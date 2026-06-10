//! OpenAI LLM provider implementation
//!
//! Supports GPT-4o, GPT-4o-mini, and other OpenAI chat completion models.

use std::time::Instant;

use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use super::{ChatMessage, CompletionRequest, CompletionResponse, CompletionStream, LlmError, LlmProvider, TokenUsage};

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// OpenAI-compatible provider.
///
/// Can be used for OpenAI's official API or any API-compatible endpoint
/// (e.g., Azure OpenAI, local proxies).
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        Self::with_base_url(api_key, OPENAI_API_BASE)
    }

    /// Create a provider with a custom base URL (for Azure, proxies, etc.).
    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()?;

        Ok(Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.into(),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let start = Instant::now();

        let messages = build_openai_messages(&request.system_prompt, &request.messages);

        let body = OpenAiChatRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(DEFAULT_TIMEOUT_SECS)
                } else {
                    LlmError::Http(e)
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LlmError::AuthError);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(LlmError::RateLimited { retry_after });
        }

        let body_text = response.text().await.map_err(LlmError::Http)?;

        if !status.is_success() {
            return Err(LlmError::ApiError {
                status: status.as_u16(),
                message: body_text,
            });
        }

        let completion: OpenAiChatResponse =
            serde_json::from_str(&body_text).map_err(|e| LlmError::ApiError {
                status: 200,
                message: format!("Failed to parse OpenAI response: {}. Body: {}", e, body_text),
            })?;

        let choice = completion
            .choices
            .into_iter()
            .next()
            .ok_or(LlmError::EmptyResponse)?;

        let content = choice.message.content.unwrap_or_default();
        if content.is_empty() {
            return Err(LlmError::EmptyResponse);
        }

        let usage = completion.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(CompletionResponse {
            content,
            usage,
            provider: self.name().to_string(),
            model: completion.model,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn complete_stream(&self, request: CompletionRequest) -> Result<CompletionStream, LlmError> {
        let messages = build_openai_messages(&request.system_prompt, &request.messages);

        let body = OpenAiChatRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,  // Enable streaming
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(DEFAULT_TIMEOUT_SECS)
                } else {
                    LlmError::Http(e)
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(LlmError::AuthError);
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(LlmError::RateLimited { retry_after });
        }
        if !status.is_success() {
            let body_text = response.text().await.map_err(LlmError::Http)?;
            return Err(LlmError::ApiError {
                status: status.as_u16(),
                message: body_text,
            });
        }

        // Parse SSE stream with line buffering to handle split chunks
        let stream = response.bytes_stream();
        let mut buf = String::new();
        let token_stream = stream.filter_map(move |chunk| {
            let mut content = String::new();
            match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    buf.push_str(&text);

                    // Process all complete lines in the buffer
                    while let Some(pos) = buf.find('\n') {
                        let line = buf.drain(..=pos).collect::<String>();
                        let line = line.trim_end();
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                continue;
                            }
                            match serde_json::from_str::<OpenAiStreamChunk>(data) {
                                Ok(chunk) => {
                                    if let Some(delta) = chunk.choices.first() {
                                        if let Some(text) = &delta.delta.content {
                                            content.push_str(text);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, data = %data, "Failed to parse SSE chunk");
                                }
                            }
                        }
                    }

                    std::future::ready(if content.is_empty() {
                        None
                    } else {
                        Some(Ok(content))
                    })
                }
                Err(e) => std::future::ready(Some(Err(LlmError::Http(e)))),
            }
        });

        Ok(Box::pin(token_stream))
    }
}

// ------------------------------------------------------------------
// OpenAI API types
// ------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn build_openai_messages(system_prompt: &str, messages: &[ChatMessage]) -> Vec<OpenAiMessage> {
    let mut result = vec![OpenAiMessage {
        role: "system".to_string(),
        content: Some(system_prompt.to_string()),
        name: None,
    }];

    for msg in messages {
        result.push(OpenAiMessage {
            role: match msg.role {
                super::MessageRole::System => "system".to_string(),
                super::MessageRole::User => "user".to_string(),
                super::MessageRole::Assistant => "assistant".to_string(),
            },
            content: Some(msg.content.clone()),
            name: msg.name.clone(),
        });
    }

    result
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_openai_messages() {
        let messages = vec![
            ChatMessage::user("Hello", Some("alice".to_string())),
            ChatMessage::assistant("Hi there!"),
        ];

        let openai_msgs = build_openai_messages("You are a bot.", &messages);
        assert_eq!(openai_msgs.len(), 3);
        assert_eq!(openai_msgs[0].role, "system");
        assert_eq!(openai_msgs[1].role, "user");
        assert_eq!(openai_msgs[1].name, Some("alice".to_string()));
        assert_eq!(openai_msgs[2].role, "assistant");
    }
}
