use super::errors::ProviderError;
use super::types::{CompletionResponse, Message, ModelConfig, Tool};
use async_trait::async_trait;

/// Metadata about a provider's capabilities
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    /// Name of the provider
    pub name: String,

    /// Whether the provider supports streaming responses
    pub supports_streaming: bool,

    /// Whether the provider supports tool/function calling
    pub supports_tools: bool,

    /// Whether the provider supports vision/image inputs
    pub supports_vision: bool,

    /// Maximum context window size in tokens
    pub max_tokens: usize,
}

/// Trait for AI providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Generate a completion from a list of messages
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation history
    /// * `tools` - Available tools for the model to use
    /// * `config` - Model configuration (temperature, etc.)
    ///
    /// # Returns
    ///
    /// Returns a `CompletionResponse` or `ProviderError`
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[Tool],
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError>;

    /// Generate a completion using the fast model
    ///
    /// This is a convenience method for operations that need speed over quality.
    /// Default implementation falls back to `complete`.
    async fn complete_fast(
        &self,
        messages: &[Message],
        tools: &[Tool],
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError> {
        self.complete(messages, tools, config).await
    }

    /// Get provider capabilities
    fn get_metadata(&self) -> ProviderMetadata;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::providers::types::Usage;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _tools: &[Tool],
            _config: &ModelConfig,
        ) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                id: "mock".to_string(),
                object: "chat.completion".to_string(),
                created: 1234567890,
                model: "mock-model".to_string(),
                choices: vec![],
                usage: Some(Usage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                }),
            })
        }

        fn get_metadata(&self) -> ProviderMetadata {
            ProviderMetadata {
                name: "mock".to_string(),
                supports_streaming: false,
                supports_tools: true,
                supports_vision: false,
                max_tokens: 4096,
            }
        }
    }

    #[tokio::test]
    async fn test_provider_trait() {
        let provider = MockProvider;
        let metadata = provider.get_metadata();
        assert_eq!(metadata.name, "mock");
        assert!(metadata.supports_tools);

        let result = provider.complete(&[], &[], &ModelConfig::default()).await;
        assert!(result.is_ok());

        // Test default implementation of complete_fast
        let result_fast = provider
            .complete_fast(&[], &[], &ModelConfig::default())
            .await;
        assert!(result_fast.is_ok());
    }
}
