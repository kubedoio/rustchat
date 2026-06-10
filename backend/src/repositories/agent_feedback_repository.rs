use sqlx::PgPool;
use uuid::Uuid;

use crate::models::agent_feedback::*;

pub struct AgentFeedbackRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AgentFeedbackRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_feedback(
        &self,
        post_id: Uuid,
        user_id: Uuid,
        feedback_type: &str,
        comment: Option<&str>,
    ) -> Result<AgentMessageFeedback, sqlx::Error> {
        sqlx::query_as::<_, AgentMessageFeedback>(
            r#"
            INSERT INTO agent_message_feedback (post_id, user_id, feedback_type, comment)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (post_id, user_id)
            DO UPDATE SET feedback_type = EXCLUDED.feedback_type, comment = EXCLUDED.comment
            RETURNING *
            "#,
        )
        .bind(post_id)
        .bind(user_id)
        .bind(feedback_type)
        .bind(comment)
        .fetch_one(self.pool)
        .await
    }

    pub async fn get_feedback_for_post(
        &self,
        post_id: Uuid,
    ) -> Result<Vec<AgentMessageFeedback>, sqlx::Error> {
        sqlx::query_as::<_, AgentMessageFeedback>(
            "SELECT * FROM agent_message_feedback WHERE post_id = $1 ORDER BY created_at DESC",
        )
        .bind(post_id)
        .fetch_all(self.pool)
        .await
    }

    pub async fn get_feedback_summary(
        &self,
        post_id: Uuid,
    ) -> Result<FeedbackSummary, sqlx::Error> {
        sqlx::query_as::<_, FeedbackSummary>(
            r#"
            SELECT
                $1 as post_id,
                COUNT(*) FILTER (WHERE feedback_type = 'positive') as positive_count,
                COUNT(*) FILTER (WHERE feedback_type = 'negative') as negative_count
            FROM agent_message_feedback
            WHERE post_id = $1
            "#,
        )
        .bind(post_id)
        .fetch_one(self.pool)
        .await
    }

    pub async fn delete_feedback(&self, post_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM agent_message_feedback WHERE post_id = $1 AND user_id = $2")
            .bind(post_id)
            .bind(user_id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    pub async fn get_agent_feedback_stats(
        &self,
        agent_id: Uuid,
    ) -> Result<AgentFeedbackStats, sqlx::Error> {
        sqlx::query_as::<_, AgentFeedbackStats>(
            r#"
            SELECT
                $1 as agent_id,
                COUNT(*) FILTER (WHERE f.feedback_type = 'positive') as total_positive,
                COUNT(*) FILTER (WHERE f.feedback_type = 'negative') as total_negative,
                COUNT(*) as total_feedback,
                COALESCE(
                    COUNT(*) FILTER (WHERE f.feedback_type = 'positive')::float / NULLIF(COUNT(*), 0),
                    0.0
                ) as feedback_ratio
            FROM agent_message_feedback f
            JOIN posts p ON p.id = f.post_id
            WHERE p.user_id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_one(self.pool)
        .await
    }
}
