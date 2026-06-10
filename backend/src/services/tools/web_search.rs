//! Web search tool using Tavily API
//!
//! Tavily is designed for AI agents and provides clean, LLM-optimized search results.
//! Docs: https://docs.tavily.com/

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::registry::{Tool, ToolError};

pub struct WebSearchTool {
    client: reqwest::Client,
    api_key: String,
}

impl WebSearchTool {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
        }
    }
}

#[derive(Serialize)]
struct TavilySearchRequest {
    query: String,
    api_key: String,
    search_depth: String,
    include_answer: bool,
    max_results: i32,
}

#[derive(Deserialize)]
struct TavilySearchResponse {
    answer: Option<String>,
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for current information, news, facts, and documentation. \
         Use this when you need up-to-date information that may not be in your knowledge base."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (1-10)",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query' parameter".to_string()))?;

        let max_results = args["max_results"].as_i64().unwrap_or(5).clamp(1, 10) as i32;

        let request = TavilySearchRequest {
            query: query.to_string(),
            api_key: self.api_key.clone(),
            search_depth: "basic".to_string(),
            include_answer: true,
            max_results,
        };

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .json(&request)
            .send()
            .await
            .map_err(|e| ToolError::Execution(format!("HTTP error: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ToolError::Execution(format!(
                "Tavily API error {}: {}",
                status.as_u16(),
                body
            )));
        }

        let body: TavilySearchResponse = response
            .json()
            .await
            .map_err(|e| ToolError::Execution(format!("JSON parse error: {}", e)))?;

        let mut output = String::new();

        if let Some(answer) = body.answer {
            output.push_str("## Summary\n");
            output.push_str(&answer);
            output.push_str("\n\n");
        }

        if !body.results.is_empty() {
            output.push_str("## Sources\n");
            for (i, result) in body.results.iter().enumerate() {
                output.push_str(&format!(
                    "{}. {}\n   URL: {}\n   Content: {}\n\n",
                    i + 1,
                    result.title,
                    result.url,
                    result.content
                ));
            }
        }

        if output.is_empty() {
            output.push_str("No results found.");
        }

        Ok(output)
    }
}
