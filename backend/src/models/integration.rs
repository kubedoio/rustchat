//! Integration models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Incoming webhook entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct IncomingWebhook {
    pub id: Uuid,
    pub team_id: Uuid,
    pub channel_id: Uuid,
    pub creator_id: Uuid,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub token: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Outgoing webhook entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct OutgoingWebhook {
    pub id: Uuid,
    pub team_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub creator_id: Uuid,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub trigger_words: Vec<String>,
    pub trigger_when: String,
    pub callback_urls: Vec<String>,
    pub content_type: Option<String>,
    pub token: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Slash command entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SlashCommand {
    pub id: Uuid,
    pub team_id: Uuid,
    pub creator_id: Uuid,
    pub trigger: String,
    pub url: String,
    pub method: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub hint: Option<String>,
    pub icon_url: Option<String>,
    pub token: String,
    pub is_active: bool,
    pub auto_complete: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bot entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Bot {
    pub id: Uuid,
    pub user_id: Uuid,
    pub owner_id: Uuid,
    pub display_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bot token entity
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BotToken {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub token: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// DTOs
#[derive(Debug, Clone, Deserialize)]
pub struct CreateIncomingWebhook {
    pub channel_id: Uuid,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutgoingWebhook {
    pub channel_id: Option<Uuid>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub trigger_words: Vec<String>,
    #[serde(default = "default_trigger_when")]
    pub trigger_when: String,
    pub callback_urls: Vec<String>,
}

fn default_trigger_when() -> String {
    "first_word".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSlashCommand {
    pub trigger: String,
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub hint: Option<String>,
}

fn default_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateBot {
    pub display_name: String,
    pub description: Option<String>,
}

/// Incoming webhook payload (from external service)
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayload {
    pub text: String,
    pub username: Option<String>,
    pub icon_url: Option<String>,
    pub channel: Option<String>,
    #[serde(default)]
    pub props: serde_json::Value,
}

/// Outgoing webhook response
#[derive(Debug, Clone, Serialize)]
pub struct OutgoingWebhookPayload {
    pub token: String,
    pub team_id: Uuid,
    pub channel_id: Uuid,
    pub channel_name: String,
    pub user_id: Uuid,
    pub user_name: String,
    pub text: String,
    pub trigger_word: String,
}

/// Command execution request
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteCommand {
    pub command: String,
    pub channel_id: Uuid,
    pub team_id: Option<Uuid>,
}

/// Command execution response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub response_type: String, // "in_channel" or "ephemeral"
    pub text: String,
    pub username: Option<String>,
    pub icon_url: Option<String>,
    pub goto_location: Option<String>,
    pub attachments: Option<serde_json::Value>,
}
