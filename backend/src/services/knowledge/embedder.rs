//! Text embedding services
//!
//! Converts text chunks into dense vector embeddings via LLM APIs.

use serde::{Deserialize, Serialize};

/// Embedding error types.
#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API error: {status} — {message}")]
    ApiError { status: u16, message: String },
    #[error("Serialization error: {0}")]
    SerializeError(#[from] serde_json::Error),
    #[error("Empty input")]
    EmptyInput,
}

/// OpenAI API embedder.
pub struct OpenAiEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
    dimensions: usize,
    base_url: String,
}

impl OpenAiEmbedder {
    pub fn new(api_key: String, model: Option<String>, dimensions: Option<usize>) -> Self {
        let model = model.unwrap_or_else(|| "text-embedding-3-small".to_string());
        let dimensions = dimensions.unwrap_or(1536);
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            dimensions,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        if texts.is_empty() {
            return Err(EmbedError::EmptyInput);
        }

        let mut all_embeddings = Vec::new();

        // OpenAI allows up to 96 texts per request
        for batch in texts.chunks(96) {
            let request = OpenAiEmbeddingRequest {
                input: batch.to_vec(),
                model: self.model.clone(),
                dimensions: self.dimensions,
            };

            let response = self
                .client
                .post(format!("{}/embeddings", self.base_url))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            let status = response.status();
            if !status.is_success() {
                let message = response.text().await.unwrap_or_default();
                return Err(EmbedError::ApiError {
                    status: status.as_u16(),
                    message,
                });
            }

            let body: OpenAiEmbeddingResponse = response.json().await?;
            for item in body.data {
                all_embeddings.push(item.embedding);
            }
        }

        Ok(all_embeddings)
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }
}

#[derive(Serialize)]
struct OpenAiEmbeddingRequest {
    input: Vec<String>,
    model: String,
    dimensions: usize,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingResponse {
    data: Vec<OpenAiEmbeddingData>,
}

#[derive(Deserialize)]
struct OpenAiEmbeddingData {
    embedding: Vec<f32>,
}
