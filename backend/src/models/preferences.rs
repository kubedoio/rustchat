//! User preferences and status models

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User status (displayed to other users)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserStatus {
    pub presence: Option<String>, // 'online', 'away', 'dnd', 'offline'
    pub text: Option<String>,
    pub emoji: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// User preferences from database
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct UserPreferences {
    pub user_id: Uuid,

    // Notification preferences
    pub notify_desktop: String,
    pub notify_push: String,
    pub notify_email: String,
    pub notify_sounds: bool,

    // DND
    pub dnd_enabled: bool,
    pub dnd_start_time: Option<NaiveTime>,
    pub dnd_end_time: Option<NaiveTime>,
    pub dnd_days: String,

    // Display
    pub message_display: String,
    pub sidebar_behavior: String,
    pub time_format: String,

    // Keywords
    pub mention_keywords: Option<Vec<String>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for updating user status
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateStatus {
    pub presence: Option<String>,
    pub text: Option<String>,
    pub emoji: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<i32>,
}

/// DTO for updating preferences
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePreferences {
    // Notifications
    pub notify_desktop: Option<String>,
    pub notify_push: Option<String>,
    pub notify_email: Option<String>,
    pub notify_sounds: Option<bool>,

    // DND
    pub dnd_enabled: Option<bool>,
    pub dnd_start_time: Option<String>,
    pub dnd_end_time: Option<String>,
    pub dnd_days: Option<String>,

    // Display
    pub message_display: Option<String>,
    pub sidebar_behavior: Option<String>,
    pub time_format: Option<String>,

    // Keywords
    pub mention_keywords: Option<Vec<String>>,
}

/// Status preset
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StatusPreset {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub emoji: String,
    pub text: String,
    pub duration_minutes: Option<i32>,
    pub is_default: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// DTO for creating status preset
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStatusPreset {
    pub emoji: String,
    pub text: String,
    pub duration_minutes: Option<i32>,
}

/// Channel notification setting
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ChannelNotificationSetting {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub notify_level: String,
    pub is_muted: bool,
    pub mute_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for updating channel notification
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateChannelNotification {
    pub notify_level: Option<String>,
    pub is_muted: Option<bool>,
    pub mute_until: Option<DateTime<Utc>>,
}
