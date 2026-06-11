//! Mock LLM provider for deterministic tests.
//!
//! The mock implements the same provider trait as real LLM backends without
//! making network calls. Tests can use plain response text, streaming chunks,
//! or an injected error to exercise agent runtime paths.

use async_trait::async_trait;
use futures_util::stream;

use super::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmError, LlmProvider, TokenUsage,
};

/// Error variants supported by [`MockLlmProvider`].
#[derive(Debug, Clone)]
pub enum MockLlmError {
    ApiError { status: u16, message: String },
    RateLimited { retry_after: u64 },
    AuthError,
    EmptyResponse,
    Timeout(u64),
    Config(String),
}

impl From<MockLlmError> for LlmError {
    fn from(error: MockLlmError) -> Self {
        match error {
            MockLlmError::ApiError { status, message } => LlmError::ApiError { status, message },
            MockLlmError::RateLimited { retry_after } => LlmError::RateLimited { retry_after },
            MockLlmError::AuthError => LlmError::AuthError,
            MockLlmError::EmptyResponse => LlmError::EmptyResponse,
            MockLlmError::Timeout(seconds) => LlmError::Timeout(seconds),
            MockLlmError::Config(message) => LlmError::Config(message),
        }
    }
}

/// Deterministic provider implementation for unit and integration tests.
#[derive(Debug, Clone)]
pub struct MockLlmProvider {
    pub response_text: String,
    pub should_error: Option<MockLlmError>,
    pub stream_chunks: Vec<String>,
    pub latency_ms: u64,
    pub usage: Option<TokenUsage>,
}

impl MockLlmProvider {
    pub fn new(response_text: impl Into<String>) -> Self {
        Self {
            response_text: response_text.into(),
            should_error: None,
            stream_chunks: Vec::new(),
            latency_ms: 0,
            usage: None,
        }
    }

    pub fn with_error(mut self, error: MockLlmError) -> Self {
        self.should_error = Some(error);
        self
    }

    pub fn with_stream_chunks(
        mut self,
        chunks: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.stream_chunks = chunks.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_usage(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        if let Some(error) = self.should_error.clone() {
            return Err(error.into());
        }

        Ok(CompletionResponse {
            content: self.response_text.clone(),
            usage: self.usage.clone(),
            provider: self.name().to_string(),
            model: request.model,
            latency_ms: self.latency_ms,
        })
    }

    async fn complete_stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream, LlmError> {
        if let Some(error) = self.should_error.clone() {
            return Err(error.into());
        }

        let chunks = if self.stream_chunks.is_empty() {
            vec![self.response_text.clone()]
        } else {
            self.stream_chunks.clone()
        };

        Ok(Box::pin(stream::iter(chunks.into_iter().map(Ok))))
    }
}

#[cfg(test)]
mod tests {
    use futures_util::TryStreamExt;

    use super::*;
    use crate::services::llm::ChatMessage;

    fn request() -> CompletionRequest {
        CompletionRequest {
            system_prompt: "You are a test assistant".to_string(),
            messages: vec![ChatMessage::user("hello", None)],
            model: "mock-model".to_string(),
            temperature: 0.0,
            max_tokens: 128,
        }
    }

    #[tokio::test]
    async fn complete_returns_configured_response() {
        let provider = MockLlmProvider::new("mock response").with_usage(TokenUsage {
            prompt_tokens: 3,
            completion_tokens: 2,
            total_tokens: 5,
        });

        let response = provider.complete(request()).await.unwrap();

        assert_eq!(response.content, "mock response");
        assert_eq!(response.provider, "mock");
        assert_eq!(response.model, "mock-model");
        assert_eq!(response.usage.unwrap().total_tokens, 5);
    }

    #[tokio::test]
    async fn complete_returns_configured_error() {
        let provider = MockLlmProvider::new("unused")
            .with_error(MockLlmError::RateLimited { retry_after: 42 });

        let error = provider.complete(request()).await.unwrap_err();

        assert!(matches!(error, LlmError::RateLimited { retry_after: 42 }));
    }

    #[tokio::test]
    async fn complete_stream_returns_configured_chunks() {
        let provider = MockLlmProvider::new("unused").with_stream_chunks(["hel", "lo"]);

        let chunks: Vec<String> = provider
            .complete_stream(request())
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(chunks, vec!["hel".to_string(), "lo".to_string()]);
    }
}
