//! Agent model and related types
//!
//! Defines the configuration, memory, and channel settings for AI agents.
//! Agents are users with entity_type = Agent. This module provides the
//! structured configuration layered on top of the users table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Capabilities bitmask for an agent, stored as JSONB in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default = "default_true")]
    pub respond_to_mentions: bool,
    #[serde(default = "default_false")]
    pub respond_to_all: bool,
    #[serde(default = "default_true")]
    pub use_memory: bool,
    #[serde(default = "default_false")]
    pub use_rag: bool,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            respond_to_mentions: true,
            respond_to_all: false,
            use_memory: true,
            use_rag: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

/// Agent configuration stored in the database.
///
/// Each agent config is linked 1:1 to a users row via `user_id`.
/// When the user is soft-deleted, the config is cascade-deleted.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentConfig {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing)]
    pub api_token_encrypted: Option<String>,
    pub temperature: f32,
    pub max_context_messages: i32,
    pub max_output_tokens: i32,
    pub capabilities: serde_json::Value,
    pub rag_enabled: bool,
    pub rag_top_k: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

/// Public-facing agent config (excludes sensitive fields like encrypted tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub provider: String,
    pub model: String,
    pub temperature: f64,
    pub max_context_messages: i32,
    pub max_output_tokens: i32,
    pub capabilities: serde_json::Value,
    pub rag_enabled: bool,
    pub rag_top_k: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
}

impl From<AgentConfig> for AgentConfigResponse {
    fn from(config: AgentConfig) -> Self {
        Self {
            id: config.id,
            user_id: config.user_id,
            title: config.title,
            description: config.description,
            system_prompt: config.system_prompt,
            provider: config.provider,
            model: config.model,
            temperature: f64::from(config.temperature),
            max_context_messages: config.max_context_messages,
            max_output_tokens: config.max_output_tokens,
            capabilities: config.capabilities,
            rag_enabled: config.rag_enabled,
            rag_top_k: config.rag_top_k,
            is_active: config.is_active,
            created_at: config.created_at,
            updated_at: config.updated_at,
            created_by: config.created_by,
        }
    }
}

/// Agent memory entry for conversation context persistence.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentMemory {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub channel_id: Uuid,
    pub memory_type: String,
    pub content: String,
    pub message_ids: Option<Vec<Uuid>>,
    pub importance_score: f32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Per-channel settings override for an agent.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentChannelSettings {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub channel_id: Uuid,
    pub is_active: bool,
    pub custom_prompt_override: Option<String>,
    pub max_context_messages_override: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new agent.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAgentRequest {
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub provider: String,
    pub model: String,
    pub api_token: Option<String>,
    pub temperature: Option<f64>,
    pub max_context_messages: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub capabilities: Option<AgentCapabilities>,
    pub rag_enabled: Option<bool>,
    pub rag_top_k: Option<i32>,
    pub channel_ids: Option<Vec<Uuid>>,
}

/// Request to update an existing agent.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateAgentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_token: Option<String>,
    pub temperature: Option<f64>,
    pub max_context_messages: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub capabilities: Option<AgentCapabilities>,
    pub rag_enabled: Option<bool>,
    pub rag_top_k: Option<i32>,
    pub is_active: Option<bool>,
}

/// Public agent summary (for listing).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AgentSummary {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub is_active: bool,
    pub channel_count: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Request to test an agent prompt.
#[derive(Debug, Clone, Deserialize)]
pub struct TestAgentRequest {
    pub message: String,
    pub channel_id: Option<Uuid>,
}

/// Response from testing an agent prompt.
#[derive(Debug, Clone, Serialize)]
pub struct TestAgentResponse {
    pub response: String,
    pub provider: String,
    pub model: String,
    pub latency_ms: u64,
}
