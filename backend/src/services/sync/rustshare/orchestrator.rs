//! RustShare sync orchestrator

use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use uuid::Uuid;

use crate::crypto;
use crate::models::knowledge::{CreateKnowledgeDocument, KnowledgeSyncSource, SyncStateUpdate};
use crate::repositories::KnowledgeRepository;
use crate::services::knowledge::extractor::ExtractorRegistry;
use crate::storage::S3Client;

use super::client::{RustShareClient, RustShareFile};

pub struct SyncOrchestrator {
    db: sqlx::PgPool,
    s3_client: S3Client,
    s3_bucket: String,
    encryption_key: String,
}

impl SyncOrchestrator {
    pub fn new(
        db: sqlx::PgPool,
        s3_client: S3Client,
        s3_bucket: String,
        encryption_key: String,
    ) -> Self {
        Self {
            db,
            s3_client,
            s3_bucket,
            encryption_key,
        }
    }

    /// Perform a full sync of a RustShare folder to a knowledge base.
    pub async fn full_sync(
        &self,
        sync_source: &KnowledgeSyncSource,
        kb_id: Uuid,
        created_by: Uuid,
    ) -> Result<SyncReport, SyncError> {
        let config: RustShareSyncConfig =
            decrypt_config(&sync_source.config_encrypted, &self.encryption_key)?;
        let client = RustShareClient::new(config.base_url, config.auth_token);
        let repo = KnowledgeRepository::new(&self.db);

        let mut report = SyncReport::default();

        // List all files in the folder (paginated)
        let mut page_token: Option<String> = None;
        loop {
            let list = client
                .list_files(&config.folder_id, None, page_token.as_deref())
                .await?;

            for file in list.files {
                match self
                    .sync_file(
                        &client,
                        &file,
                        kb_id,
                        sync_source.team_id,
                        sync_source.id,
                        created_by,
                    )
                    .await
                {
                    Ok(action) => match action {
                        SyncAction::Created => report.created += 1,
                        SyncAction::Updated => report.updated += 1,
                        SyncAction::Unchanged => report.unchanged += 1,
                    },
                    Err(e) => {
                        tracing::error!(error = %e, file_id = %file.id, "File sync failed");
                        report.failed += 1;
                    }
                }
            }

            page_token = list.next_page_token;
            if page_token.is_none() {
                break;
            }
        }

        // Update sync source state
        let state = SyncStateUpdate {
            last_sync_at: Some(Utc::now()),
            last_sync_status: Some(if report.failed > 0 {
                "partial".to_string()
            } else {
                "success".to_string()
            }),
            last_sync_error: None,
            next_sync_at: None,
            document_count: Some(report.total() as i32),
        };
        repo.update_sync_state(sync_source.id, sync_source.team_id, &state)
            .await
            .map_err(SyncError::Database)?;

        Ok(report)
    }

    /// Sync a single file (create, update, or skip).
    async fn sync_file(
        &self,
        client: &RustShareClient,
        file: &RustShareFile,
        kb_id: Uuid,
        team_id: Uuid,
        sync_source_id: Uuid,
        created_by: Uuid,
    ) -> Result<SyncAction, SyncError> {
        let repo = KnowledgeRepository::new(&self.db);

        // Check for existing document by external_id
        let existing = sqlx::query_as::<_, crate::models::knowledge::KnowledgeDocument>(
            "SELECT * FROM knowledge_documents WHERE external_id = $1 AND team_id = $2",
        )
        .bind(&file.id)
        .bind(team_id)
        .fetch_optional(&self.db)
        .await
        .map_err(SyncError::Database)?;

        if let Some(ref doc) = existing {
            // If etag matches, skip
            if doc.external_etag.as_deref() == Some(&file.etag) {
                return Ok(SyncAction::Unchanged);
            }
            // Otherwise delete old document and re-sync
            repo.delete_document(doc.id, team_id)
                .await
                .map_err(SyncError::Database)?;
        }

        // Download file
        let data = client.download_file(&file.id).await?;
        let content_hash = format!("{:x}", sha2::Sha256::digest(&data));

        // Deduplication by content hash
        if let Some(_dup) = repo
            .get_document_by_hash(&content_hash, team_id)
            .await
            .map_err(SyncError::Database)?
        {
            return Ok(SyncAction::Unchanged);
        }

        // Upload to S3
        let doc_id = Uuid::new_v4();
        let key = format!("knowledge/{}/{}/{}/{}", team_id, kb_id, doc_id, file.name);
        self.s3_client
            .upload(&key, data.clone(), &file.mime_type)
            .await
            .map_err(|e| SyncError::Storage(e.to_string()))?;

        // Create document record
        let doc = repo
            .create_document(&CreateKnowledgeDocument {
                id: Some(doc_id),
                knowledge_base_id: kb_id,
                team_id,
                title: file.name.clone(),
                source_url: Some(file.download_url.clone()),
                source_type: "rustshare".to_string(),
                s3_key: key,
                s3_bucket: self.s3_bucket.clone(),
                content_hash,
                mime_type: file.mime_type.clone(),
                size_bytes: file.size_bytes,
                extracted_text: None,
                external_id: Some(file.id.clone()),
                external_etag: Some(file.etag.clone()),
                external_modified_at: Some(
                    chrono::DateTime::parse_from_rfc3339(&file.modified_at)
                        .map_err(|_| SyncError::DateParse)?
                        .with_timezone(&chrono::Utc),
                ),
                sync_source_id: Some(sync_source_id),
                created_by,
            })
            .await
            .map_err(SyncError::Database)?;

        // Background extraction
        let db_pool = self.db.clone();
        let doc_id_bg = doc.id;
        let mime_bg = file.mime_type.clone();
        let data_bg = data.clone();
        tokio::spawn(async move {
            let repo = KnowledgeRepository::new(&db_pool);
            let registry = ExtractorRegistry::default_registry();
            match registry.extract(&data_bg, &mime_bg) {
                Ok(text) => {
                    if let Err(e) = repo
                        .update_document_extracted(doc_id_bg, team_id, &text)
                        .await
                    {
                        tracing::error!(
                            error = %e,
                            doc_id = %doc_id_bg,
                            "RustShare extraction storage failed"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        doc_id = %doc_id_bg,
                        "RustShare extraction failed"
                    );
                }
            }
        });

        Ok(if existing.is_some() {
            SyncAction::Updated
        } else {
            SyncAction::Created
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RustShareSyncConfig {
    pub base_url: String,
    pub auth_token: String,
    pub folder_id: String,
    pub knowledge_base_id: Option<uuid::Uuid>,
    pub recursive: bool,
}

fn decrypt_config(encrypted: &str, encryption_key: &str) -> Result<RustShareSyncConfig, SyncError> {
    let json = match crypto::decrypt(encrypted, encryption_key) {
        Ok(config_json) => config_json,
        Err(decrypt_error) => {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(encrypted)
                .map_err(|legacy_error| {
                    SyncError::Config(format!(
                        "Encrypted config decrypt failed ({decrypt_error}); legacy base64 decode failed ({legacy_error})"
                    ))
                })?;
            String::from_utf8(decoded)
                .map_err(|e| SyncError::Config(format!("UTF-8 decode failed: {}", e)))?
        }
    };

    serde_json::from_str(&json).map_err(|e| SyncError::Config(format!("JSON parse failed: {}", e)))
}

#[derive(Debug, Default, Serialize)]
pub struct SyncReport {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub failed: usize,
}

impl SyncReport {
    pub fn total(&self) -> usize {
        self.created + self.updated + self.unchanged + self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_json() -> String {
        serde_json::json!({
            "base_url": "https://share.example.com",
            "auth_token": "secret-token",
            "folder_id": "folder-123",
            "knowledge_base_id": null,
            "recursive": true
        })
        .to_string()
    }

    #[test]
    fn decrypt_config_reads_encrypted_json() {
        let key = "test-encryption-key";
        let encrypted = crypto::encrypt(&config_json(), key).expect("encrypt config");

        let config = decrypt_config(&encrypted, key).expect("decrypt config");

        assert_eq!(config.base_url, "https://share.example.com");
        assert_eq!(config.auth_token, "secret-token");
        assert_eq!(config.folder_id, "folder-123");
        assert!(config.recursive);
    }

    #[test]
    fn decrypt_config_preserves_legacy_base64_fallback() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(config_json());

        let config = decrypt_config(&encoded, "wrong-key").expect("decode legacy config");

        assert_eq!(config.base_url, "https://share.example.com");
        assert_eq!(config.auth_token, "secret-token");
        assert_eq!(config.folder_id, "folder-123");
        assert!(config.recursive);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SyncAction {
    Created,
    Updated,
    Unchanged,
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("RustShare API error: {0}")]
    RustShare(#[from] super::client::RustShareError),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Config error: {0}")]
    Config(String),
    #[error("Date parse error")]
    DateParse,
}
