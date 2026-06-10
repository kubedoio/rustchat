//! LLM provider registry
//!
//! Maintains a mapping of provider names to their implementations.
//! Initialized at startup from environment configuration.

use std::collections::HashMap;
use std::sync::Arc;

use super::{LlmError, LlmProvider};

/// Registry of configured LLM providers.
///
/// Multiple agents can share the same provider instance,
/// allowing connection pool reuse.
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LlmProvider>>,
}

impl ProviderRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider under the given name.
    pub fn register(&mut self, name: impl Into<String>, provider: Arc<dyn LlmProvider>) {
        let name = name.into();
        tracing::info!(provider = %name, "Registered LLM provider");
        self.providers.insert(name, provider);
    }

    /// Get a provider by name.
    pub fn get(&self, name: &str) -> Result<Arc<dyn LlmProvider>, LlmError> {
        self.providers
            .get(name)
            .cloned()
            .ok_or_else(|| LlmError::ProviderNotConfigured(name.to_string()))
    }

    /// Check if a provider is registered.
    pub fn has(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }

    /// List all registered provider names.
    pub fn list(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::super::{CompletionRequest, CompletionResponse, LlmError};
    use super::*;

    struct MockProvider {
        name: &'static str,
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                content: "mock response".to_string(),
                usage: None,
                provider: self.name.to_string(),
                model: "mock".to_string(),
                latency_ms: 0,
            })
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ProviderRegistry::new();
        let provider = Arc::new(MockProvider { name: "test" });
        registry.register("test", provider.clone());

        assert!(registry.has("test"));
        assert!(!registry.has("missing"));

        let got = registry.get("test").unwrap();
        assert_eq!(got.name(), "test");
    }

    #[test]
    fn test_get_missing_provider() {
        let registry = ProviderRegistry::new();
        let result = registry.get("missing");
        assert!(matches!(result, Err(LlmError::ProviderNotConfigured(_))));
    }

    #[test]
    fn test_list_providers() {
        let mut registry = ProviderRegistry::new();
        registry.register("openai", Arc::new(MockProvider { name: "openai" }));
        registry.register("anthropic", Arc::new(MockProvider { name: "anthropic" }));

        let names = registry.list();
        assert_eq!(names.len(), 2);
    }
}
