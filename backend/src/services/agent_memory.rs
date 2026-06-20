//! Agent memory service
//!
//! Provides context building and memory storage for AI agents.

use std::sync::Arc;

use crate::error::{ApiResult, AppError};
use crate::models::agent::AgentMemory;
use crate::models::knowledge::SearchFilter;
use crate::repositories::{AgentRepository, KnowledgeRepository};
use crate::services::knowledge::embedder::OpenAiEmbedder;
use crate::services::knowledge::vector_store::PgVectorStore;
use crate::services::llm::ChatMessage;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

pub struct AgentMemoryService {
    db: PgPool,
    embedder: Option<Arc<OpenAiEmbedder>>,
    vector_store: Option<Arc<PgVectorStore>>,
}

impl AgentMemoryService {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            embedder: None,
            vector_store: None,
        }
    }

    pub fn with_rag(
        mut self,
        embedder: Arc<OpenAiEmbedder>,
        vector_store: Arc<PgVectorStore>,
    ) -> Self {
        self.embedder = Some(embedder);
        self.vector_store = Some(vector_store);
        self
    }

    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, channel_id = %channel_id))]
    pub async fn build_context(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        max_messages: i32,
        use_rag: bool,
        query: &str,
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

        // RAG context injection
        if use_rag {
            if let (Some(embedder), Some(_vector_store)) = (&self.embedder, &self.vector_store) {
                let kb_repo = KnowledgeRepository::new(&self.db);
                let kb_assignments = kb_repo
                    .list_agent_knowledge_bases(agent_id)
                    .await
                    .map_err(AppError::Database)?;

                if !kb_assignments.is_empty() {
                    let query_embedding =
                        embedder.embed(&[query.to_string()]).await.map_err(|e| {
                            AppError::ExternalService(format!("Embedding failed: {}", e))
                        })?;
                    let query_vector = query_embedding
                        .into_iter()
                        .next()
                        .ok_or_else(|| AppError::ExternalService("Empty embedding".to_string()))?;

                    let mut rag_parts = vec!["## Relevant Knowledge".to_string()];
                    for assignment in kb_assignments {
                        let filter = SearchFilter {
                            team_id: assignment.team_id,
                            knowledge_base_id: Some(assignment.knowledge_base_id),
                            document_id: None,
                        };
                        let chunks = kb_repo
                            .search_chunks_hybrid(query, &query_vector, assignment.top_k, &filter)
                            .await
                            .map_err(AppError::Database)?;

                        for chunk in chunks {
                            if let Some(threshold) = assignment.relevance_threshold {
                                if chunk.similarity < threshold {
                                    continue;
                                }
                            }
                            let source = chunk
                                .section_title
                                .as_ref()
                                .map(|s| format!(" [{}]", s))
                                .unwrap_or_default();
                            rag_parts.push(format!(
                                "- [{}]{source}\n{}",
                                chunk.document_title, chunk.chunk_text
                            ));
                        }
                    }

                    if rag_parts.len() > 1 {
                        messages.push(ChatMessage::system(rag_parts.join("\n\n")));
                    }
                }
            }
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
        let content = format!(
            "Responded to post {}: {}",
            trigger_post_id, response_content
        );

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
