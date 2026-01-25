//! File model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// File entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FileInfo {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub post_id: Option<Uuid>,
    pub name: String,
    pub key: String,
    pub mime_type: String,
    pub size: i64,
    pub backend: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub has_thumbnail: bool,
    pub thumbnail_key: Option<String>,
    pub sha256: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response for file upload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub id: Uuid,
    pub name: String,
    pub mime_type: String,
    pub size: i64,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

/// Response for presigned upload URL
#[derive(Debug, Clone, Serialize)]
pub struct PresignedUploadUrl {
    pub upload_url: String,
    pub file_key: String,
    pub expires_in: u64,
}
