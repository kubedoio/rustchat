use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentMessageFeedback {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user_id: Uuid,
    pub feedback_type: String,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateFeedbackRequest {
    pub feedback_type: String, // "positive" or "negative"
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FeedbackSummary {
    pub post_id: Uuid,
    pub positive_count: i64,
    pub negative_count: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentFeedbackStats {
    pub agent_id: Uuid,
    pub total_positive: i64,
    pub total_negative: i64,
    pub total_feedback: i64,
    pub feedback_ratio: f64,
}
