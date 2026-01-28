use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum MiroTalkMode {
    #[default]
    Disabled,
    Sfu,
    P2p,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JoinBehavior {
    #[default]
    NewTab,
    EmbedIframe,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MiroTalkConfig {
    pub is_active: bool,
    pub mode: MiroTalkMode,
    pub base_url: String,
    pub api_key_secret: String,
    pub default_room_prefix: Option<String>,
    pub join_behavior: JoinBehavior,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
}

impl MiroTalkConfig {
    pub fn is_enabled(&self) -> bool {
        self.mode != MiroTalkMode::Disabled && !self.base_url.is_empty()
    }
}
