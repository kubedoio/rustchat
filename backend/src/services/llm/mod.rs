//! LLM provider abstraction layer
//!
//! Defines the `LlmProvider` trait and common types for interacting with
//! large language model APIs (OpenAI, Anthropic, Ollama, etc.).

use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod openai;
pub mod registry;

pub use openai::OpenAiProvider;
pub use registry::ProviderRegistry;

/// Role of a message in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    /// Optional name to identify the speaker in multi-user contexts.
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
        }
    }

    pub fn user(content: impl Into<String>, name: Option<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
        }
    }
}

/// Request to complete a conversation.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

/// Response from an LLM completion.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    pub usage: Option<TokenUsage>,
    pub provider: String,
    pub model: String,
    pub latency_ms: u64,
}

/// Token usage statistics from an LLM response.
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Errors that can occur when calling an LLM provider.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limit exceeded. Retry after: {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Invalid API key or authentication failed")]
    AuthError,

    #[error("Provider returned empty response")]
    EmptyResponse,

    #[error("Request timed out after {0}s")]
    Timeout(u64),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Invalid configuration: {0}")]
    Config(String),
}

/// A stream of completion tokens.
pub type CompletionStream =
    Pin<Box<dyn futures_util::Stream<Item = Result<String, LlmError>> + Send>>;

/// Trait for LLM providers.
///
/// Implementors must be `Send + Sync` to allow shared use across async tasks.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider name for metrics and logging.
    fn name(&self) -> &'static str;

    /// Send a conversation to the LLM and return the complete text response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// (Phase 2) Stream tokens from the LLM as they are generated.
    ///
    /// Default implementation returns an error indicating streaming is not supported.
    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream, LlmError> {
        Err(LlmError::Config(
            "Streaming not implemented for this provider".to_string(),
        ))
    }
}
