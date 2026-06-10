//! Knowledge base REST API endpoints
//!
//! Provides CRUD operations for knowledge bases, document upload and management,
//! and agent↔KB assignment.

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    api::AppState,
    auth::AuthUser,
    error::{ApiResult, AppError},
    models::knowledge::*,
    repositories::{AgentRepository, KnowledgeRepository, TeamRepository},
    services::knowledge::{
        embedder::OpenAiEmbedder, extractor::ExtractorRegistry, indexer::IndexerService,
        vector_store::PgVectorStore,
    },
    services::sync::rustshare::orchestrator::SyncOrchestrator,
};

// ------------------------------------------------------------------
// Router
// ------------------------------------------------------------------

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/bases",
            get(list_knowledge_bases).post(create_knowledge_base),
        )
        .route(
            "/bases/:id",
            get(get_knowledge_base)
                .put(update_knowledge_base)
                .delete(delete_knowledge_base),
        )
        .route(
            "/bases/:id/documents",
            get(list_documents).post(upload_document),
        )
        .route(
            "/documents/:doc_id",
            get(get_document).delete(delete_document),
        )
        .route("/documents/:doc_id/download", get(download_document))
        .route(
            "/sync-sources",
            get(list_sync_sources).post(create_sync_source),
        )
        .route(
            "/sync-sources/:id",
            get(get_sync_source)
                .put(update_sync_source)
                .delete(delete_sync_source),
        )
        .route("/sync-sources/:id/sync", post(trigger_sync))
        .route("/sync/rustshare", post(handle_rustshare_webhook))
}

// ------------------------------------------------------------------
// Authorization helpers
// ------------------------------------------------------------------

async fn resolve_user_team_id(db: &sqlx::PgPool, user_id: Uuid) -> ApiResult<Uuid> {
    TeamRepository::new(db)
        .get_first_team_for_user(user_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::BadRequest("User has no team".to_string()))
}

fn require_admin(auth: &AuthUser) -> ApiResult<()> {
    if !auth.has_role("system_admin") && !auth.has_role("org_admin") && !auth.has_role("team_admin")
    {
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }
    Ok(())
}

// ------------------------------------------------------------------
// Knowledge Base Handlers
// ------------------------------------------------------------------

async fn create_knowledge_base(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateKnowledgeBaseRequest>,
) -> ApiResult<(StatusCode, Json<KnowledgeBase>)> {
    require_admin(&auth)?;

    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let kb = repo
        .create_knowledge_base(
            team_id,
            &req.name,
            req.description.as_deref(),
            req.embedding_model
                .as_deref()
                .unwrap_or("text-embedding-3-small"),
            req.embedding_dimensions.unwrap_or(1536),
            req.chunk_size.unwrap_or(512),
            req.chunk_overlap.unwrap_or(50),
            auth.user_id,
        )
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(kb)))
}

async fn list_knowledge_bases(
    auth: AuthUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<KnowledgeBaseSummary>>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let kbs = repo
        .list_knowledge_bases(team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(kbs))
}

async fn get_knowledge_base(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<KnowledgeBase>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let kb = repo
        .get_knowledge_base(id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Knowledge base not found".to_string()))?;

    Ok(Json(kb))
}

async fn update_knowledge_base(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateKnowledgeBaseRequest>,
) -> ApiResult<Json<KnowledgeBase>> {
    require_admin(&auth)?;

    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let description = match req.description {
        Some(ref d) if d.is_empty() => Some(None),
        Some(ref d) => Some(Some(d.as_str())),
        None => None,
    };

    let kb = repo
        .update_knowledge_base(
            id,
            team_id,
            req.name.as_deref(),
            description,
            req.embedding_model.as_deref(),
            req.embedding_dimensions,
            req.chunk_size,
            req.chunk_overlap,
            req.is_active,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(kb))
}

async fn delete_knowledge_base(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    require_admin(&auth)?;

    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    repo.delete_knowledge_base(id, team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------
// Document Handlers
// ------------------------------------------------------------------

#[tracing::instrument(skip(state, multipart), fields(kb_id = %kb_id))]
pub async fn upload_document(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(kb_id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let kb = repo
        .get_knowledge_base(kb_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Knowledge base not found".to_string()))?;

    // Extract file from multipart
    let field = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
        .ok_or_else(|| AppError::BadRequest("No file provided".to_string()))?;

    let filename = field.file_name().unwrap_or("upload").to_string();
    let mime_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let data = field
        .bytes()
        .await
        .map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?
        .to_vec();

    // Validate file size (max 50MB)
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest(format!(
            "File too large: {} bytes (max {}MB)",
            data.len(),
            MAX_FILE_SIZE / 1024 / 1024
        )));
    }

    // Validate MIME type
    let allowed_mime_types = [
        "text/plain",
        "text/markdown",
        "text/html",
        "text/x-rust",
        "text/x-python",
        "application/pdf",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ];
    if !allowed_mime_types.contains(&mime_type.as_str()) {
        return Err(AppError::BadRequest(format!(
            "Unsupported file type: {}. Allowed: {:?}",
            mime_type, allowed_mime_types
        )));
    }

    // Compute hash
    let hash = Sha256::digest(&data);
    let hash_hex = hex::encode(hash);

    // Deduplication check
    if let Some(existing) = repo
        .get_document_by_hash(&hash_hex, team_id)
        .await
        .map_err(AppError::Database)?
    {
        return Ok((StatusCode::OK, Json(existing)));
    }

    let doc_id = Uuid::new_v4();
    let key = format!("knowledge/{}/{}/{}/{}", kb.team_id, kb.id, doc_id, filename);

    // Create document record
    let doc = repo
        .create_document(&CreateKnowledgeDocument {
            id: Some(doc_id),
            knowledge_base_id: kb_id,
            team_id: kb.team_id,
            title: filename.clone(),
            source_url: None,
            source_type: "upload".to_string(),
            s3_key: key.clone(),
            s3_bucket: state.config.s3_bucket.clone(),
            content_hash: hash_hex.clone(),
            mime_type: mime_type.clone(),
            size_bytes: data.len() as i64,
            extracted_text: None,
            external_id: None,
            external_etag: None,
            external_modified_at: None,
            sync_source_id: None,
            created_by: auth.user_id,
        })
        .await
        .map_err(AppError::Database)?;

    // Upload to S3
    state
        .s3_client
        .upload(&key, data.clone(), &mime_type)
        .await?;

    // Spawn background extraction + indexing pipeline
    let db_pool = state.db.clone();
    let doc_id_for_indexing = doc.id;
    let team_id_for_indexing = team_id;
    let data_for_extraction = data.clone();
    let mime_for_extraction = mime_type.clone();

    // Get OpenAI API key for embedder
    let embedder_api_key = std::env::var("RUSTCHAT_OPENAI_API_KEY").ok();

    tokio::spawn(async move {
        let repo = KnowledgeRepository::new(&db_pool);
        let registry = ExtractorRegistry::default_registry();

        // Step 1: Extract text
        match registry.extract(&data_for_extraction, &mime_for_extraction) {
            Ok(text) => {
                if let Err(e) = repo
                    .update_document_extracted(doc_id_for_indexing, team_id_for_indexing, &text)
                    .await
                {
                    tracing::error!(
                        error = %e,
                        doc_id = %doc_id_for_indexing,
                        "Text extraction storage failed"
                    );
                    return;
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    doc_id = %doc_id_for_indexing,
                    "Text extraction failed"
                );
                return;
            }
        }

        // Step 2: Index (only if we have an OpenAI API key)
        if let Some(api_key) = embedder_api_key {
            let embedder = Arc::new(OpenAiEmbedder::new(api_key, None, None));
            let vector_store = Arc::new(PgVectorStore::new(&db_pool));
            let indexer = IndexerService::new(db_pool.clone(), embedder, vector_store);

            if let Err(e) = indexer
                .index_document(doc_id_for_indexing, team_id_for_indexing)
                .await
            {
                tracing::error!(
                    error = %e,
                    doc_id = %doc_id_for_indexing,
                    "Document indexing failed"
                );
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(doc)))
}

async fn list_documents(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(kb_id): Path<Uuid>,
) -> ApiResult<Json<Vec<KnowledgeDocument>>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    // Verify KB exists and belongs to user's team
    repo.get_knowledge_base(kb_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Knowledge base not found".to_string()))?;

    let docs = repo
        .list_documents(kb_id, team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(docs))
}

async fn get_document(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> ApiResult<Json<KnowledgeDocument>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let doc = repo
        .get_document(doc_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    Ok(Json(doc))
}

async fn delete_document(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let doc = repo
        .get_document(doc_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    // Delete from S3
    if let Err(e) = state.s3_client.delete(&doc.s3_key).await {
        tracing::warn!(
            error = %e,
            s3_key = %doc.s3_key,
            "Failed to delete document from S3"
        );
    }

    repo.delete_document(doc_id, team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn download_document(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let doc = repo
        .get_document(doc_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Document not found".to_string()))?;

    let url = state
        .s3_client
        .presigned_download_url(&doc.s3_key, 3600)
        .await?;

    Ok(Json(serde_json::json!({
        "url": url,
        "expires_in": 3600,
    })))
}

// ------------------------------------------------------------------
// Agent ↔ KB Handlers
// ------------------------------------------------------------------

pub async fn assign_kb_to_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignKnowledgeBaseRequest>,
) -> ApiResult<Json<AgentKnowledgeBase>> {
    require_admin(&auth)?;

    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    // Verify agent exists and extract its user_id (agent_id FK target)
    let agent_repo = AgentRepository::new(&state.db);
    let agent = agent_repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    // Verify KB exists and belongs to user's team
    let kb_repo = KnowledgeRepository::new(&state.db);
    kb_repo
        .get_knowledge_base(req.knowledge_base_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Knowledge base not found".to_string()))?;

    let mapping = kb_repo
        .assign_kb_to_agent(
            agent.user_id,
            req.knowledge_base_id,
            req.top_k.unwrap_or(3),
            req.relevance_threshold,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(mapping))
}

pub async fn list_agent_knowledge_bases(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<AgentKnowledgeBaseDetail>>> {
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    // Verify agent exists
    let agent_repo = AgentRepository::new(&state.db);
    agent_repo
        .get_config_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Agent not found".to_string()))?;

    let kbs: Vec<AgentKnowledgeBaseDetail> = sqlx::query_as(
        r#"
        SELECT
            akb.agent_id,
            akb.knowledge_base_id,
            akb.top_k,
            akb.relevance_threshold,
            kb.name as knowledge_base_name,
            kb.description as knowledge_base_description
        FROM agent_knowledge_bases akb
        JOIN knowledge_bases kb ON kb.id = akb.knowledge_base_id
        WHERE akb.agent_id = $1 AND kb.team_id = $2
        ORDER BY akb.created_at DESC
        "#,
    )
    .bind(id)
    .bind(team_id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(kbs))
}

pub async fn unassign_kb_from_agent(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, kb_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    require_admin(&auth)?;

    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    // Verify KB exists and belongs to user's team
    let kb_repo = KnowledgeRepository::new(&state.db);
    kb_repo
        .get_knowledge_base(kb_id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Knowledge base not found".to_string()))?;

    kb_repo
        .unassign_kb_from_agent(id, kb_id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------------
// Sync Source Handlers
// ------------------------------------------------------------------

async fn create_sync_source(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<CreateSyncSourceRequest>,
) -> ApiResult<(StatusCode, Json<KnowledgeSyncSource>)> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let config_json = serde_json::to_string(&req.config)
        .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;
    // TODO: encrypt config_json with RUSTCHAT_ENCRYPTION_KEY
    let config_encrypted = base64::engine::general_purpose::STANDARD.encode(config_json);

    let source = repo
        .create_sync_source(
            team_id,
            &req.name,
            &req.source_type,
            &config_encrypted,
            &req.sync_mode.unwrap_or_else(|| "push".to_string()),
            req.sync_interval_minutes,
        )
        .await
        .map_err(AppError::Database)?;

    Ok((StatusCode::CREATED, Json(source)))
}

async fn list_sync_sources(
    auth: AuthUser,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<KnowledgeSyncSource>>> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let sources = repo
        .list_sync_sources(team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(sources))
}

async fn get_sync_source(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<KnowledgeSyncSource>> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let source = repo
        .get_sync_source(id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Sync source not found".to_string()))?;

    Ok(Json(source))
}

async fn update_sync_source(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateSyncSourceRequest>,
) -> ApiResult<Json<KnowledgeSyncSource>> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);

    let config_encrypted = if let Some(config) = req.config {
        let config_json = serde_json::to_string(&config)
            .map_err(|e| AppError::BadRequest(format!("Invalid config: {}", e)))?;
        Some(base64::engine::general_purpose::STANDARD.encode(config_json))
    } else {
        None
    };

    let source = repo
        .update_sync_source(
            id,
            team_id,
            req.name.as_deref(),
            config_encrypted.as_deref(),
            req.sync_mode.as_deref(),
            req.sync_interval_minutes,
            req.is_active,
        )
        .await
        .map_err(AppError::Database)?;

    Ok(Json(source))
}

async fn delete_sync_source(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    repo.delete_sync_source(id, team_id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn trigger_sync(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<crate::services::sync::rustshare::orchestrator::SyncReport>> {
    require_admin(&auth)?;
    let team_id = resolve_user_team_id(&state.db, auth.user_id).await?;

    let repo = KnowledgeRepository::new(&state.db);
    let source = repo
        .get_sync_source(id, team_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("Sync source not found".to_string()))?;

    // Parse config to get kb_id
    let config_bytes = base64::engine::general_purpose::STANDARD
        .decode(&source.config_encrypted)
        .map_err(|e| AppError::BadRequest(format!("Invalid config encoding: {}", e)))?;
    let config_json: serde_json::Value = serde_json::from_slice(&config_bytes)
        .map_err(|e| AppError::BadRequest(format!("Invalid config JSON: {}", e)))?;
    let kb_id = config_json
        .get("knowledge_base_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| {
            AppError::BadRequest(
                "Missing or invalid knowledge_base_id in sync source config".to_string(),
            )
        })?;

    let orchestrator = SyncOrchestrator::new(
        state.db.clone(),
        state.s3_client.clone(),
        state.config.s3_bucket.clone(),
    );
    let report = orchestrator
        .full_sync(&source, kb_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(report))
}

// ------------------------------------------------------------------
// RustShare Webhook Handler
// ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RustShareWebhookPayload {
    pub event: String,
    pub webhook_id: String,
    pub timestamp: String,
    pub folder_id: String,
    pub file: RustShareWebhookFile,
}

#[derive(Debug, Deserialize)]
pub struct RustShareWebhookFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub etag: String,
    pub modified_at: String,
}

async fn handle_rustshare_webhook(
    State(_state): State<AppState>,
    Json(payload): Json<RustShareWebhookPayload>,
) -> ApiResult<StatusCode> {
    tracing::info!(
        event = %payload.event,
        webhook_id = %payload.webhook_id,
        folder_id = %payload.folder_id,
        file_id = %payload.file.id,
        "Received RustShare webhook"
    );

    // TODO: Implement incremental sync based on webhook events
    // For now, just acknowledge receipt

    Ok(StatusCode::OK)
}
