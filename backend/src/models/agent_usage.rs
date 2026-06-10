use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentUsageLog {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub channel_id: Uuid,
    pub trigger_type: String,
    pub tokens_input: i32,
    pub tokens_output: i32,
    pub latency_ms: i32,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentUsageSummary {
    pub agent_id: Uuid,
    pub total_invocations: i64,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub avg_latency_ms: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentDailyUsage {
    pub date: NaiveDate,
    pub invocations: i64,
    pub tokens_input: i64,
    pub tokens_output: i64,
}
