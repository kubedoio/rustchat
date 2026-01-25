//! Post (message) model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Post entity (message)
/// Post entity (message)
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub root_post_id: Option<Uuid>,
    pub message: String,
    pub props: serde_json::Value,
    pub file_ids: Vec<Uuid>,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub reply_count: i32, // use i32 or i64, DB is INT (i32) usually, but BIGINT is i64
    pub last_reply_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub seq: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Reaction {
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub emoji_name: String,
    pub created_at: DateTime<Utc>,
}

/// Aggregated reaction for responses
#[derive(Debug, Clone, Serialize)]
pub struct ReactionResponse {
    pub emoji: String,
    pub count: i32,
    pub users: Vec<Uuid>,
}

/// DTO for creating a post
#[derive(Debug, Clone, Deserialize)]
pub struct CreatePost {
    pub message: String,
    pub root_post_id: Option<Uuid>,
    #[serde(default)]
    pub props: Option<serde_json::Value>,
    #[serde(default)]
    pub file_ids: Vec<Uuid>,
}

/// DTO for updating a post
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePost {
    pub message: String,
}

/// DTO for adding a reaction
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReaction {
    pub emoji_name: String,
}

/// Post with author info for responses
#[derive(Debug, Clone, Serialize)]
pub struct PostWithAuthor {
    #[serde(flatten)]
    pub post: Post,
    pub author_username: String,
    pub author_display_name: Option<String>,
}

/// Post response with user info (for API responses)
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PostResponse {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub root_post_id: Option<Uuid>,
    pub message: String,
    pub props: serde_json::Value,
    pub file_ids: Vec<Uuid>,
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub reply_count: i32,
    pub last_reply_at: Option<DateTime<Utc>>,
    // User info from JOIN
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    // reply_count: Option<i64> - Removed, using direct field
    #[sqlx(skip)]
    pub files: Vec<crate::models::FileUploadResponse>,
    #[sqlx(skip)]
    pub reactions: Vec<ReactionResponse>,
    #[sqlx(skip)]
    pub is_saved: bool,
    #[sqlx(skip)]
    pub client_msg_id: Option<String>,
    #[sqlx(default)]
    pub seq: i64,
}
