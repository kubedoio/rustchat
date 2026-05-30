//! Activity (notification) model for user activity feed

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Types of activity that can appear in the feed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    Mention,
    Reply,
    Reaction,
    Dm,
    ThreadReply,
}

/// Activity entity - represents a notification for a user
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub r#type: ActivityType,
    pub actor_id: Uuid,
    pub channel_id: Uuid,
    pub team_id: Uuid,
    pub post_id: Uuid,
    pub root_id: Option<Uuid>,
    pub message_text: Option<String>,
    pub reaction: Option<String>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// Activity response with joined user/channel info for API
#[derive(Debug, Clone, Serialize)]
pub struct ActivityResponse {
    pub id: Uuid,
    pub r#type: ActivityType,
    pub actor_id: Uuid,
    pub actor_username: String,
    pub actor_avatar_url: Option<String>,
    pub channel_id: Uuid,
    pub channel_name: String,
    pub team_id: Uuid,
    pub team_name: String,
    pub post_id: Uuid,
    pub root_id: Option<Uuid>,
    pub message_text: Option<String>,
    pub reaction: Option<String>,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

/// Query parameters for fetching activities
#[derive(Debug, Clone, Default)]
pub struct ActivityQuery {
    pub cursor: Option<Uuid>,
    pub limit: i64,
    pub activity_type: Option<String>, // Comma-separated types
    pub unread_only: bool,
}

/// Response for activity feed endpoint
#[derive(Debug, Clone, Serialize)]
pub struct ActivityFeedResponse {
    pub order: Vec<String>,
    pub activities: std::collections::HashMap<String, ActivityResponse>,
    pub unread_count: i64,
    pub next_cursor: Option<String>,
}

/// Request to mark activities as read
#[derive(Debug, Clone, Deserialize)]
pub struct MarkReadRequest {
    pub activity_ids: Vec<Uuid>,
}

impl ActivityType {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivityType::Mention => "mention",
            ActivityType::Reply => "reply",
            ActivityType::Reaction => "reaction",
            ActivityType::Dm => "dm",
            ActivityType::ThreadReply => "thread_reply",
        }
    }

    /// Parse activity type from string
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mention" => Some(ActivityType::Mention),
            "reply" => Some(ActivityType::Reply),
            "reaction" => Some(ActivityType::Reaction),
            "dm" => Some(ActivityType::Dm),
            "thread_reply" => Some(ActivityType::ThreadReply),
            _ => None,
        }
    }
}
