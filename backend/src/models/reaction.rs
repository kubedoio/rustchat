//! Post reactions model
//!
//! Reactions are emoji responses to posts.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A reaction to a post
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Reaction {
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub emoji_name: String,
    pub create_at: i64,
}

/// DTO for creating a reaction
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReactionRequest {
    pub post_id: String,
    pub emoji_name: String,
}

/// DTO for reaction response (Mattermost compatible)
#[derive(Debug, Clone, Serialize)]
pub struct ReactionResponse {
    pub user_id: String,
    pub post_id: String,
    pub emoji_name: String,
    pub create_at: i64,
}

impl From<Reaction> for ReactionResponse {
    fn from(r: Reaction) -> Self {
        Self {
            user_id: crate::mattermost_compat::id::encode_mm_id(r.user_id),
            post_id: crate::mattermost_compat::id::encode_mm_id(r.post_id),
            emoji_name: r.emoji_name,
            create_at: r.create_at,
        }
    }
}

/// Reaction count for a post
#[derive(Debug, Clone, Serialize)]
pub struct ReactionCount {
    pub emoji_name: String,
    pub count: i64,
}
