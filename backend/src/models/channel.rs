//! Channel model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "channel_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Public,
    Private,
    Direct,
    Group,
}

impl Default for ChannelType {
    fn default() -> Self {
        Self::Public
    }
}

/// Channel entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub team_id: Uuid,
    #[sqlx(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    pub header: Option<String>,
    pub is_archived: bool,
    pub creator_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Channel member
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ChannelMember {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub notify_props: serde_json::Value,
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,

    // Joined fields (optional)
    #[sqlx(default)]
    pub username: Option<String>,
    #[sqlx(default)]
    pub display_name: Option<String>,
    #[sqlx(default)]
    pub avatar_url: Option<String>,
    #[sqlx(default)]
    pub presence: Option<String>,
}

/// DTO for creating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct CreateChannel {
    pub team_id: Uuid,
    pub name: String,
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    #[serde(default)]
    pub channel_type: ChannelType,
    pub target_user_id: Option<Uuid>,
}

/// DTO for updating a channel
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannel {
    pub display_name: Option<String>,
    pub purpose: Option<String>,
    pub header: Option<String>,
}
