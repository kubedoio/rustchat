//! Construction of the optional AI agent runtime.
//!
//! The agent runtime is built exclusively from typed configuration
//! ([`crate::config::AgentRuntimeConfig`]). No environment variables are
//! read here; `Config::load` parses the environment exactly once.
//!
//! When no LLM provider is configured the runtime is `None`: the
//! application starts normally and agent features are disabled.

use std::sync::Arc;

use sqlx::PgPool;

use crate::config::Config;
use crate::realtime::WsHub;
use crate::services::agent_runtime::AgentRuntime;
use crate::services::knowledge::embedder::OpenAiEmbedder;
use crate::services::knowledge::vector_store::PgVectorStore;
use crate::services::llm::{OpenAiProvider, ProviderRegistry};

/// Build the agent runtime from typed configuration.
///
/// Returns `None` when no LLM provider is configured (the runtime is
/// optional and must never prevent startup).
pub fn build_agent_runtime(
    config: &Config,
    db: &PgPool,
    ws_hub: &Arc<WsHub>,
) -> Option<Arc<AgentRuntime>> {
    let mut provider_registry = ProviderRegistry::new();
    let mut has_provider = false;

    if let Some(api_key) = config.agents.openai_api_key.clone() {
        match OpenAiProvider::new(api_key.clone()) {
            Ok(provider) => {
                provider_registry.register("openai", Arc::new(provider));
                has_provider = true;
                tracing::info!("OpenAI LLM provider registered");
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to initialize OpenAI provider");
            }
        }
    }

    if !has_provider {
        tracing::info!("No LLM providers configured; agent runtime disabled");
        return None;
    }

    let embedder = config
        .agents
        .openai_api_key
        .clone()
        .map(|key| Arc::new(OpenAiEmbedder::new(key, None, None)));
    let vector_store = Some(Arc::new(PgVectorStore::new(db)));

    let tool_registry = build_tool_registry(config);

    Some(Arc::new(AgentRuntime::new(
        db.clone(),
        ws_hub.clone(),
        Arc::new(provider_registry),
        embedder,
        vector_store,
        tool_registry,
    )))
}

/// Build the server-side tool registry.
///
/// Tools are registered only when their backing credentials are configured;
/// an empty registry is represented as `None` so the runtime skips tool
/// schemas entirely.
fn build_tool_registry(
    config: &Config,
) -> Option<Arc<crate::services::tools::registry::ToolRegistry>> {
    let tool_registry = Arc::new(crate::services::tools::registry::ToolRegistry::new());

    if let Some(tavily_key) = config.agents.tavily_api_key.clone() {
        tool_registry.register(Arc::new(
            crate::services::tools::web_search::WebSearchTool::new(tavily_key),
        ));
        tracing::info!("Web search tool registered");
    }

    if tool_registry.is_empty() {
        None
    } else {
        Some(tool_registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        // Minimal config; agent keys are unset.
        serde_json::from_str(
            r#"{"environment":"development","database_url":"postgres://fake:fake@localhost:5432/fake","jwt_secret":"test-secret","encryption_key":"test-encryption-key"}"#,
        )
        .expect("minimal config")
    }

    #[tokio::test]
    async fn agent_runtime_is_none_without_provider_key() {
        let config = test_config();
        assert!(config.agents.openai_api_key.is_none());
        assert!(build_agent_runtime(
            &config,
            &PgPool::connect_lazy("postgres://fake:fake@localhost:5432/fake").unwrap(),
            &WsHub::new()
        )
        .is_none());
    }
}
