//! Tool registry

use std::sync::Arc;
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;

/// Error type for tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Execution failed: {0}")]
    Execution(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// A tool that an agent can invoke.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique tool name (snake_case).
    fn name(&self) -> &'static str;
    /// Human-readable description for the LLM.
    fn description(&self) -> &'static str;
    /// JSON Schema for the tool's parameters.
    fn schema(&self) -> serde_json::Value;
    /// Execute the tool with the given arguments.
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError>;
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: DashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    pub fn register(&self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).map(|t| t.clone())
    }

    pub fn list(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.iter().map(|t| t.clone()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Build a JSON array of tool schemas for the LLM system prompt.
    pub fn schemas_json(&self) -> serde_json::Value {
        let schemas: Vec<serde_json::Value> = self
            .list()
            .iter()
            .map(|t| {
                json!({
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.schema(),
                })
            })
            .collect();
        serde_json::Value::Array(schemas)
    }

    /// Build a formatted tool instructions string for the system prompt.
    pub fn tool_instructions(&self) -> String {
        if self.tools.is_empty() {
            return String::new();
        }

        let mut instructions = String::from(
            "\n\n## Available Tools\n\n\
            You have access to the following tools. To use a tool, respond with a <tool_call> block:\n\n\
            <tool_call name=\"TOOL_NAME\">\n\
            { \"arg1\": \"value1\", \"arg2\": \"value2\" }\n\
            </tool_call>\n\n\
            You can call multiple tools in parallel by including multiple <tool_call> blocks.\n\
            After tool results are provided, continue with your final response.\n\n"
        );

        for tool in self.list() {
            instructions.push_str(&format!(
                "### {}\n{}\nSchema: {}\n\n",
                tool.name(),
                tool.description(),
                serde_json::to_string_pretty(&tool.schema()).unwrap_or_default()
            ));
        }

        instructions
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
