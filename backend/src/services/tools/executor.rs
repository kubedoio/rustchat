//! Tool executor — runs the LLM ↔ tool loop

use std::sync::Arc;
use regex::Regex;

use crate::services::llm::{ChatMessage, CompletionRequest, LlmError, LlmProvider};

use super::registry::{ToolError, ToolRegistry};

pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    max_iterations: usize,
}

impl ToolExecutor {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            max_iterations: 5,
        }
    }

    /// Run the LLM with tool access, looping until no more tool calls or max iterations.
    pub async fn execute_with_tools(
        &self,
        provider: &dyn LlmProvider,
        mut request: CompletionRequest,
    ) -> Result<String, ToolExecutionError> {
        let tool_instructions = self.registry.tool_instructions();
        if !tool_instructions.is_empty() {
            request.system_prompt.push_str(&tool_instructions);
        }

        for iteration in 0..self.max_iterations {
            tracing::debug!(iteration, "Tool execution loop");

            let response = provider.complete(request.clone()).await?;
            let content = response.content;

            // Parse tool calls from the response
            let tool_calls = parse_tool_calls(&content);

            if tool_calls.is_empty() {
                // No tool calls — return the response
                return Ok(content);
            }

            // Append the assistant's message (with tool calls) to the conversation
            request.messages.push(ChatMessage::assistant(content.clone()));

            // Execute tools in parallel
            let mut tool_results = Vec::new();
            for call in tool_calls {
                match self.registry.get(&call.name) {
                    Some(tool) => {
                        match tool.execute(call.arguments).await {
                            Ok(result) => {
                                tool_results.push(format!(
                                    "<tool_result name=\"{}\">\n{}\n</tool_result>",
                                    call.name, result
                                ));
                            }
                            Err(e) => {
                                tool_results.push(format!(
                                    "<tool_result name=\"{}\" error=\"true\">\n{}\n</tool_result>",
                                    call.name, e
                                ));
                            }
                        }
                    }
                    None => {
                        tool_results.push(format!(
                            "<tool_result name=\"{}\" error=\"true\">\nTool not found\n</tool_result>",
                            call.name
                        ));
                    }
                }
            }

            // Append tool results as a system/tool message
            let results_text = tool_results.join("\n\n");
            request.messages.push(ChatMessage::system(format!(
                "Tool results:\n{}",
                results_text
            )));
        }

        // Max iterations reached — do one final call without tools
        tracing::warn!("Max tool iterations reached");
        let response = provider.complete(request).await?;
        Ok(response.content)
    }
}

/// A parsed tool call.
struct ParsedToolCall {
    name: String,
    arguments: serde_json::Value,
}

/// Parse <tool_call> blocks from LLM response.
fn parse_tool_calls(content: &str) -> Vec<ParsedToolCall> {
    let mut calls = Vec::new();
    // Regex to match <tool_call name="...">{...}</tool_call>
    let re = Regex::new(r#"<tool_call\s+name="([^"]+)">\s*(\{[\s\S]*?\})\s*</tool_call>"#)
        .expect("Invalid regex");

    for cap in re.captures_iter(content) {
        let name = cap[1].to_string();
        let args_str = &cap[2];
        match serde_json::from_str(args_str) {
            Ok(arguments) => calls.push(ParsedToolCall { name, arguments }),
            Err(e) => {
                tracing::warn!(error = %e, args = %args_str, "Failed to parse tool call arguments");
            }
        }
    }

    calls
}

#[derive(Debug, thiserror::Error)]
pub enum ToolExecutionError {
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),
    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),
    #[error("Max iterations exceeded")]
    MaxIterations,
}
