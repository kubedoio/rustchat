//! RustShare API client

use serde::Deserialize;
use std::time::Duration;

/// Hard cap on a single downloaded file's size.
///
/// Downloads are buffered fully in memory before streaming to S3; without a
/// cap, a remote listing a huge file (or a compromised server) OOMs the
/// backend. The orchestrator skips larger files with a logged warning.
pub const MAX_DOWNLOAD_BYTES: i64 = 100 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct RustShareClient {
    http: reqwest::Client,
    base_url: String,
    auth_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RustShareFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub etag: String,
    pub modified_at: String, // ISO 8601
    pub download_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RustShareFileList {
    pub files: Vec<RustShareFile>,
    pub next_page_token: Option<String>,
}

impl RustShareClient {
    pub fn new(base_url: String, auth_token: String) -> Self {
        // Bounded client: without timeouts a stalled RustShare connection
        // hangs the sync task indefinitely. The overall timeout is generous
        // because legitimate large downloads take a while; the size cap in
        // `download_file` bounds memory, this bounds time.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token,
        }
    }

    /// List files in a folder, optionally filtered by modification time.
    pub async fn list_files(
        &self,
        folder_id: &str,
        modified_since: Option<&str>,
        page_token: Option<&str>,
    ) -> Result<RustShareFileList, RustShareError> {
        let mut url = format!("{}/api/v1/folders/{}/files", self.base_url, folder_id);
        let mut params = Vec::new();
        if let Some(since) = modified_since {
            params.push(("modified_since", since));
        }
        if let Some(token) = page_token {
            params.push(("page_token", token));
        }
        if !params.is_empty() {
            url = format!(
                "{}?{}",
                url,
                serde_urlencoded::to_string(&params).unwrap_or_default()
            );
        }

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RustShareError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        let list: RustShareFileList = response.json().await?;
        Ok(list)
    }

    /// Download a file's content.
    ///
    /// Fails with [`RustShareError::FileTooLarge`] when the response declares
    /// (or streams) more than [`MAX_DOWNLOAD_BYTES`]; the body is never
    /// buffered beyond that cap.
    pub async fn download_file(&self, file_id: &str) -> Result<Vec<u8>, RustShareError> {
        let url = format!("{}/api/v1/files/{}/download", self.base_url, file_id);
        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(RustShareError::ApiError {
                status: status.as_u16(),
                body,
            });
        }

        // Pre-check declared size, then bound the actual read: a lying or
        // chunked response must not be able to OOM the process either.
        if let Some(len) = response.content_length() {
            if len as i64 > MAX_DOWNLOAD_BYTES {
                return Err(RustShareError::FileTooLarge {
                    declared: len as i64,
                    limit: MAX_DOWNLOAD_BYTES,
                });
            }
        }

        let mut data: Vec<u8> = Vec::new();
        let mut stream = response;
        while let Some(chunk) = stream.chunk().await? {
            if (data.len() + chunk.len()) as i64 > MAX_DOWNLOAD_BYTES {
                return Err(RustShareError::FileTooLarge {
                    declared: -1, // unknown; exceeded while streaming
                    limit: MAX_DOWNLOAD_BYTES,
                });
            }
            data.extend_from_slice(&chunk);
        }
        Ok(data)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RustShareError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error {status}: {body}")]
    ApiError { status: u16, body: String },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("file too large: declared {declared} bytes, limit {limit}")]
    FileTooLarge { declared: i64, limit: i64 },
}
