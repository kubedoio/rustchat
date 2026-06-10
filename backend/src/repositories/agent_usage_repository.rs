use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::agent_usage::*;

pub struct AgentUsageRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AgentUsageRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn log_usage(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        trigger_type: &str,
        tokens_input: i32,
        tokens_output: i32,
        latency_ms: i32,
        model: &str,
    ) -> Result<AgentUsageLog, sqlx::Error> {
        sqlx::query_as::<_, AgentUsageLog>(
            r#"
            INSERT INTO agent_usage_logs
                (agent_id, channel_id, trigger_type, tokens_input, tokens_output, latency_ms, model)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(channel_id)
        .bind(trigger_type)
        .bind(tokens_input)
        .bind(tokens_output)
        .bind(latency_ms)
        .bind(model)
        .fetch_one(self.pool)
        .await
    }

    pub async fn get_summary(
        &self,
        agent_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<AgentUsageSummary, sqlx::Error> {
        sqlx::query_as::<_, AgentUsageSummary>(
            r#"
            SELECT
                $1 as agent_id,
                COUNT(*) as total_invocations,
                COALESCE(SUM(tokens_input), 0) as total_tokens_input,
                COALESCE(SUM(tokens_output), 0) as total_tokens_output,
                COALESCE(AVG(latency_ms)::bigint, 0) as avg_latency_ms
            FROM agent_usage_logs
            WHERE agent_id = $1 AND created_at >= $2
            "#,
        )
        .bind(agent_id)
        .bind(since)
        .fetch_one(self.pool)
        .await
    }

    pub async fn get_daily_usage(
        &self,
        agent_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<AgentDailyUsage>, sqlx::Error> {
        sqlx::query_as::<_, AgentDailyUsage>(
            r#"
            SELECT
                DATE(created_at) as date,
                COUNT(*) as invocations,
                COALESCE(SUM(tokens_input), 0) as tokens_input,
                COALESCE(SUM(tokens_output), 0) as tokens_output
            FROM agent_usage_logs
            WHERE agent_id = $1 AND created_at >= $2
            GROUP BY DATE(created_at)
            ORDER BY date DESC
            "#,
        )
        .bind(agent_id)
        .bind(since)
        .fetch_all(self.pool)
        .await
    }
}
