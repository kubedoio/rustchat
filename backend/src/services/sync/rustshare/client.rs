//! RustShare API client

use serde::Deserialize;

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
        Self {
            http: reqwest::Client::new(),
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

        Ok(response.bytes().await?.to_vec())
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
}
