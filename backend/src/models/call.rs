use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Call {
    pub id: Uuid,
    pub channel_id: Option<Uuid>,
    pub r#type: String, // 'audio', 'video', 'screen'
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub owner_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CallParticipant {
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub muted: bool,
    pub raised_hand: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCall {
    pub channel_id: Uuid,
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallSession {
    #[serde(flatten)]
    pub call: Call,
    pub participants: Vec<CallParticipant>,
}
