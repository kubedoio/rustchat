//! Agent repository
//!
//! Provides CRUD operations for agent_configs, agent_memories, and
//! agent_channel_settings. All operations use compile-time checked sqlx queries.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::agent::{
    AgentCapabilities, AgentChannelSettings, AgentConfig, AgentMemory, AgentSummary,
};

pub struct AgentRepository<'a> {
    pool: &'a PgPool,
}

impl<'a> AgentRepository<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    // ------------------------------------------------------------------
    // Agent Config
    // ------------------------------------------------------------------

    /// Create a new agent configuration.
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, api_token_encrypted, capabilities), fields(user_id = %user_id))]
    pub async fn create_config(
        &self,
        user_id: Uuid,
        title: &str,
        description: Option<&str>,
        system_prompt: &str,
        provider: &str,
        model: &str,
        api_token_encrypted: Option<&str>,
        temperature: f64,
        max_context_messages: i32,
        max_output_tokens: i32,
        capabilities: &AgentCapabilities,
        rag_enabled: bool,
        rag_top_k: i32,
        created_by: Uuid,
    ) -> Result<AgentConfig, sqlx::Error> {
        let capabilities_json = serde_json::to_value(capabilities).unwrap_or_default();

        sqlx::query_as::<_, AgentConfig>(
            r#"
            INSERT INTO agent_configs (
                user_id, title, description, system_prompt,
                provider, model, api_token_encrypted, temperature,
                max_context_messages, max_output_tokens, capabilities,
                rag_enabled, rag_top_k, created_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(description)
        .bind(system_prompt)
        .bind(provider)
        .bind(model)
        .bind(api_token_encrypted)
        .bind(temperature)
        .bind(max_context_messages)
        .bind(max_output_tokens)
        .bind(capabilities_json)
        .bind(rag_enabled)
        .bind(rag_top_k)
        .bind(created_by)
        .fetch_one(self.pool)
        .await
    }

    /// Get agent config by its own ID.
    pub async fn get_config_by_id(&self, id: Uuid) -> Result<Option<AgentConfig>, sqlx::Error> {
        sqlx::query_as::<_, AgentConfig>("SELECT * FROM agent_configs WHERE id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await
    }

    /// Get agent config by linked user ID.
    pub async fn get_config_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<AgentConfig>, sqlx::Error> {
        sqlx::query_as::<_, AgentConfig>("SELECT * FROM agent_configs WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(self.pool)
            .await
    }

    /// List all agent configs with optional filters.
    pub async fn list_configs(
        &self,
        is_active: Option<bool>,
    ) -> Result<Vec<AgentConfig>, sqlx::Error> {
        let rows = if let Some(active) = is_active {
            sqlx::query_as::<_, AgentConfig>(
                "SELECT * FROM agent_configs WHERE is_active = $1 ORDER BY created_at DESC",
            )
            .bind(active)
            .fetch_all(self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AgentConfig>("SELECT * FROM agent_configs ORDER BY created_at DESC")
                .fetch_all(self.pool)
                .await?
        };
        Ok(rows)
    }

    /// List agent summaries (public-facing, no sensitive fields).
    pub async fn list_agent_summaries(
        &self,
        is_active: Option<bool>,
    ) -> Result<Vec<AgentSummary>, sqlx::Error> {
        let query = match is_active {
            Some(active) => {
                sqlx::query_as::<_, AgentSummary>(
                    r#"
                    SELECT
                        ac.id,
                        ac.user_id,
                        u.username,
                        u.display_name,
                        u.avatar_url,
                        ac.title,
                        ac.provider,
                        ac.model,
                        ac.is_active,
                        COUNT(acs.channel_id) as channel_count,
                        ac.created_at
                    FROM agent_configs ac
                    JOIN users u ON u.id = ac.user_id
                    LEFT JOIN agent_channel_settings acs ON acs.agent_id = ac.user_id
                    WHERE ac.is_active = $1
                    GROUP BY ac.id, u.username, u.display_name, u.avatar_url
                    ORDER BY ac.created_at DESC
                    "#,
                )
                .bind(active)
                .fetch_all(self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, AgentSummary>(
                    r#"
                    SELECT
                        ac.id,
                        ac.user_id,
                        u.username,
                        u.display_name,
                        u.avatar_url,
                        ac.title,
                        ac.provider,
                        ac.model,
                        ac.is_active,
                        COUNT(acs.channel_id) as channel_count,
                        ac.created_at
                    FROM agent_configs ac
                    JOIN users u ON u.id = ac.user_id
                    LEFT JOIN agent_channel_settings acs ON acs.agent_id = ac.user_id
                    GROUP BY ac.id, u.username, u.display_name, u.avatar_url
                    ORDER BY ac.created_at DESC
                    "#,
                )
                .fetch_all(self.pool)
                .await
            }
        };
        query
    }

    /// Update agent config. Only updates fields that are Some.
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self), fields(id = %id))]
    pub async fn update_config(
        &self,
        id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        system_prompt: Option<&str>,
        provider: Option<&str>,
        model: Option<&str>,
        api_token_encrypted: Option<Option<&str>>,
        temperature: Option<f64>,
        max_context_messages: Option<i32>,
        max_output_tokens: Option<i32>,
        capabilities: Option<&AgentCapabilities>,
        rag_enabled: Option<bool>,
        rag_top_k: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<AgentConfig, sqlx::Error> {
        let existing = self
            .get_config_by_id(id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let title = title.unwrap_or(&existing.title);
        let description = match description {
            Some(d) => d,
            None => existing.description.as_deref(),
        };
        let system_prompt = system_prompt.unwrap_or(&existing.system_prompt);
        let provider = provider.unwrap_or(&existing.provider);
        let model = model.unwrap_or(&existing.model);
        let api_token = match api_token_encrypted {
            Some(t) => t,
            None => existing.api_token_encrypted.as_deref(),
        };
        let temperature = temperature.unwrap_or(f64::from(existing.temperature));
        let max_context_messages = max_context_messages.unwrap_or(existing.max_context_messages);
        let max_output_tokens = max_output_tokens.unwrap_or(existing.max_output_tokens);
        let capabilities_json = match capabilities {
            Some(c) => serde_json::to_value(c).unwrap_or(existing.capabilities),
            None => existing.capabilities,
        };
        let rag_enabled = rag_enabled.unwrap_or(existing.rag_enabled);
        let rag_top_k = rag_top_k.unwrap_or(existing.rag_top_k);
        let is_active = is_active.unwrap_or(existing.is_active);

        sqlx::query_as::<_, AgentConfig>(
            r#"
            UPDATE agent_configs
            SET
                title = $1,
                description = $2,
                system_prompt = $3,
                provider = $4,
                model = $5,
                api_token_encrypted = $6,
                temperature = $7,
                max_context_messages = $8,
                max_output_tokens = $9,
                capabilities = $10,
                rag_enabled = $11,
                rag_top_k = $12,
                is_active = $13
            WHERE id = $14
            RETURNING *
            "#,
        )
        .bind(title)
        .bind(description)
        .bind(system_prompt)
        .bind(provider)
        .bind(model)
        .bind(api_token)
        .bind(temperature)
        .bind(max_context_messages)
        .bind(max_output_tokens)
        .bind(capabilities_json)
        .bind(rag_enabled)
        .bind(rag_top_k)
        .bind(is_active)
        .bind(id)
        .fetch_one(self.pool)
        .await
    }

    /// Soft-delete an agent by marking the linked user as deleted.
    /// The CASCADE on agent_configs.user_id will clean up the config.
    #[tracing::instrument(skip(self), fields(user_id = %user_id))]
    pub async fn delete_agent(&self, user_id: Uuid, deleted_by: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE users
            SET deleted_at = NOW(), deleted_by = $1
            WHERE id = $2 AND entity_type = 'agent'
            "#,
        )
        .bind(deleted_by)
        .bind(user_id)
        .execute(self.pool)
        .await
        .map(|_| ())
    }

    // ------------------------------------------------------------------
    // Agent Memories
    // ------------------------------------------------------------------

    /// Store a new memory entry.
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, channel_id = %channel_id))]
    pub async fn create_memory(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        memory_type: &str,
        content: &str,
        message_ids: Option<&[Uuid]>,
        importance_score: f64,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<AgentMemory, sqlx::Error> {
        sqlx::query_as::<_, AgentMemory>(
            r#"
            INSERT INTO agent_memories
                (agent_id, channel_id, memory_type, content, message_ids, importance_score, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (agent_id, channel_id, memory_type, content)
            DO UPDATE SET
                updated_at = NOW(),
                importance_score = EXCLUDED.importance_score,
                expires_at = EXCLUDED.expires_at
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(channel_id)
        .bind(memory_type)
        .bind(content)
        .bind(message_ids)
        .bind(importance_score)
        .bind(expires_at)
        .fetch_one(self.pool)
        .await
    }

    /// Retrieve memories for an agent in a channel.
    pub async fn get_memories(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        limit: i32,
    ) -> Result<Vec<AgentMemory>, sqlx::Error> {
        sqlx::query_as::<_, AgentMemory>(
            r#"
            SELECT * FROM agent_memories
            WHERE agent_id = $1
              AND channel_id = $2
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY importance_score DESC, created_at DESC
            LIMIT $3
            "#,
        )
        .bind(agent_id)
        .bind(channel_id)
        .bind(limit)
        .fetch_all(self.pool)
        .await
    }

    /// Delete a specific memory entry.
    pub async fn delete_memory(&self, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM agent_memories WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Delete a specific memory entry only when it belongs to the supplied agent.
    pub async fn delete_memory_for_agent(
        &self,
        agent_id: Uuid,
        id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM agent_memories WHERE id = $1 AND agent_id = $2")
            .bind(id)
            .bind(agent_id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Clean up expired memories.
    #[tracing::instrument(skip(self))]
    pub async fn cleanup_expired_memories(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM agent_memories WHERE expires_at IS NOT NULL AND expires_at < NOW()",
        )
        .execute(self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    // ------------------------------------------------------------------
    // Agent Channel Settings
    // ------------------------------------------------------------------

    /// Add an agent to a channel with optional settings overrides.
    #[tracing::instrument(skip(self), fields(agent_id = %agent_id, channel_id = %channel_id))]
    pub async fn add_agent_to_channel(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
        is_active: bool,
        custom_prompt_override: Option<&str>,
        max_context_messages_override: Option<i32>,
    ) -> Result<AgentChannelSettings, sqlx::Error> {
        sqlx::query_as::<_, AgentChannelSettings>(
            r#"
            INSERT INTO agent_channel_settings
                (agent_id, channel_id, is_active, custom_prompt_override, max_context_messages_override)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (agent_id, channel_id)
            DO UPDATE SET
                is_active = EXCLUDED.is_active,
                custom_prompt_override = EXCLUDED.custom_prompt_override,
                max_context_messages_override = EXCLUDED.max_context_messages_override,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(agent_id)
        .bind(channel_id)
        .bind(is_active)
        .bind(custom_prompt_override)
        .bind(max_context_messages_override)
        .fetch_one(self.pool)
        .await
    }

    /// Remove an agent from a channel.
    pub async fn remove_agent_from_channel(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM agent_channel_settings WHERE agent_id = $1 AND channel_id = $2")
            .bind(agent_id)
            .bind(channel_id)
            .execute(self.pool)
            .await
            .map(|_| ())
    }

    /// Get channel settings for an agent.
    pub async fn get_channel_settings(
        &self,
        agent_id: Uuid,
        channel_id: Uuid,
    ) -> Result<Option<AgentChannelSettings>, sqlx::Error> {
        sqlx::query_as::<_, AgentChannelSettings>(
            "SELECT * FROM agent_channel_settings WHERE agent_id = $1 AND channel_id = $2",
        )
        .bind(agent_id)
        .bind(channel_id)
        .fetch_optional(self.pool)
        .await
    }

    /// List all channels an agent is assigned to.
    pub async fn list_agent_channels(
        &self,
        agent_id: Uuid,
    ) -> Result<Vec<AgentChannelSettings>, sqlx::Error> {
        sqlx::query_as::<_, AgentChannelSettings>(
            "SELECT * FROM agent_channel_settings WHERE agent_id = $1",
        )
        .bind(agent_id)
        .fetch_all(self.pool)
        .await
    }

    /// List all active agents assigned to a channel.
    pub async fn list_channel_agents(
        &self,
        channel_id: Uuid,
    ) -> Result<Vec<AgentConfig>, sqlx::Error> {
        sqlx::query_as::<_, AgentConfig>(
            r#"
            SELECT ac.*
            FROM agent_configs ac
            JOIN agent_channel_settings acs ON acs.agent_id = ac.user_id
            WHERE acs.channel_id = $1
              AND ac.is_active = TRUE
              AND acs.is_active = TRUE
            "#,
        )
        .bind(channel_id)
        .fetch_all(self.pool)
        .await
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_capabilities_default() {
        let caps = AgentCapabilities::default();
        assert!(caps.respond_to_mentions);
        assert!(!caps.respond_to_all);
        assert!(caps.use_memory);
        assert!(!caps.use_rag);
    }

    #[test]
    fn test_agent_capabilities_serde_roundtrip() {
        let caps = AgentCapabilities {
            respond_to_mentions: true,
            respond_to_all: true,
            use_memory: false,
            use_rag: true,
        };
        let json = serde_json::to_string(&caps).unwrap();
        let decoded: AgentCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.respond_to_mentions, caps.respond_to_mentions);
        assert_eq!(decoded.respond_to_all, caps.respond_to_all);
        assert_eq!(decoded.use_memory, caps.use_memory);
        assert_eq!(decoded.use_rag, caps.use_rag);
    }
}
