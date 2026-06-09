//! Agent memory service
//!
//! Provides context building and memory storage for AI agents.

use crate::error::{ApiResult, AppError};
use crate::models::agent::AgentMemory;
use crate::repositories::AgentRepository;
use crate::services::llm::ChatMessage;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

pub struct AgentMemoryService {
    db: PgPool,
}

impl AgentMemoryService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, channel_id = %channel_id))]
    pub async fn build_context(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        max_messages: i32,
    ) -> ApiResult<Vec<ChatMessage>> {
        let mut messages = Vec::new();

        // Fetch memories and prepend a system summary if any exist
        let repo = AgentRepository::new(&self.db);
        let memories = repo
            .get_memories(agent_id, channel_id, 10)
            .await
            .map_err(AppError::Database)?;

        if !memories.is_empty() {
            let summary = format_memories(&memories);
            messages.push(ChatMessage::system(summary));
        }

        // Fetch recent channel messages
        #[derive(Debug, FromRow)]
        struct ChannelMessage {
            #[allow(dead_code)]
            user_id: Uuid,
            username: Option<String>,
            message: String,
            #[allow(dead_code)]
            created_at: DateTime<Utc>,
        }

        let rows: Vec<ChannelMessage> = sqlx::query_as::<_, ChannelMessage>(
            r#"
            SELECT p.user_id, u.username, p.message, p.created_at
            FROM posts p
            JOIN users u ON u.id = p.user_id
            WHERE p.channel_id = $1
              AND p.deleted_at IS NULL
            ORDER BY p.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(channel_id)
        .bind(max_messages)
        .fetch_all(&self.db)
        .await
        .map_err(AppError::Database)?;

        // Reverse so oldest first
        for row in rows.into_iter().rev() {
            messages.push(ChatMessage::user(
                format!(
                    "{}: {}",
                    row.username.as_deref().unwrap_or("unknown"),
                    row.message
                ),
                row.username,
            ));
        }

        Ok(messages)
    }

    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, channel_id = %channel_id))]
    pub async fn store_turn(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        trigger_post_id: Uuid,
        response_content: &str,
    ) -> ApiResult<()> {
        let repo = AgentRepository::new(&self.db);
        let content = format!("Responded to post {}: {}", trigger_post_id, response_content);

        repo.create_memory(
            agent_id,
            channel_id,
            "conversation_turn",
            &content,
            Some(&[trigger_post_id]),
            0.5,
            None,
        )
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }
}

fn format_memories(memories: &[AgentMemory]) -> String {
    let mut parts = vec!["Previous memories:".to_string()];
    for m in memories {
        parts.push(format!("- [{}] {}", m.memory_type, m.content));
    }
    parts.join("\n")
}
