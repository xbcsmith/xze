//! Provider factory for dynamic provider instantiation

use super::{
    anthropic::AnthropicProvider, base::Provider, config::AiConfig, copilot::CopilotProvider,
    errors::ProviderError, ollama::OllamaProvider, openai::OpenAiProvider,
};
use std::sync::Arc;

/// Factory for creating AI provider instances
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from configuration
    ///
    /// # Arguments
    ///
    /// * `config` - AI configuration containing provider settings
    ///
    /// # Returns
    ///
    /// Returns a boxed `Provider` trait object
    ///
    /// # Errors
    ///
    /// Returns `ProviderError::Config` if:
    /// - Provider name is unknown
    /// - Required configuration is missing
    /// - Configuration is invalid
    pub fn create(config: &AiConfig) -> Result<Arc<dyn Provider>, ProviderError> {
        match config.provider.as_str() {
            "ollama" => {
                let ollama_config = config.ollama.as_ref().ok_or_else(|| {
                    ProviderError::Config("Ollama configuration missing".to_string())
                })?;
                let provider = OllamaProvider::new(ollama_config.clone(), config.model.clone())?;
                Ok(Arc::new(provider))
            }
            "openai" => {
                let openai_config = config.openai.as_ref().ok_or_else(|| {
                    ProviderError::Config("OpenAI configuration missing".to_string())
                })?;
                let provider = OpenAiProvider::new(
                    openai_config.clone(),
                    config.model.clone(),
                    config.fast_model.clone(),
                )?;
                Ok(Arc::new(provider))
            }
            "anthropic" => {
                let anthropic_config = config.anthropic.as_ref().ok_or_else(|| {
                    ProviderError::Config("Anthropic configuration missing".to_string())
                })?;
                let provider = AnthropicProvider::new(
                    anthropic_config.clone(),
                    config.model.clone(),
                    config.fast_model.clone(),
                )?;
                Ok(Arc::new(provider))
            }
            "copilot" => {
                let copilot_config = config.copilot.as_ref().ok_or_else(|| {
                    ProviderError::Config("Copilot configuration missing".to_string())
                })?;
                let provider = CopilotProvider::new(copilot_config.clone(), config.model.clone())?;
                Ok(Arc::new(provider))
            }
            name => Err(ProviderError::Config(format!(
                "Unknown provider: {}. Supported providers: ollama, openai, anthropic, copilot",
                name
            ))),
        }
    }

    /// Create a fast provider (uses fast_model if available)
    pub fn create_fast(config: &AiConfig) -> Result<Arc<dyn Provider>, ProviderError> {
        let model = config
            .fast_model
            .clone()
            .unwrap_or_else(|| config.model.clone());

        match config.provider.as_str() {
            "ollama" => {
                let ollama_config = config.ollama.as_ref().ok_or_else(|| {
                    ProviderError::Config("Ollama configuration missing".to_string())
                })?;
                let provider = OllamaProvider::new(ollama_config.clone(), model)?;
                Ok(Arc::new(provider))
            }
            "openai" => {
                let openai_config = config.openai.as_ref().ok_or_else(|| {
                    ProviderError::Config("OpenAI configuration missing".to_string())
                })?;
                let provider = OpenAiProvider::new(openai_config.clone(), model, None)?;
                Ok(Arc::new(provider))
            }
            "anthropic" => {
                let anthropic_config = config.anthropic.as_ref().ok_or_else(|| {
                    ProviderError::Config("Anthropic configuration missing".to_string())
                })?;
                let provider = AnthropicProvider::new(anthropic_config.clone(), model, None)?;
                Ok(Arc::new(provider))
            }
            "copilot" => {
                let copilot_config = config.copilot.as_ref().ok_or_else(|| {
                    ProviderError::Config("Copilot configuration missing".to_string())
                })?;
                let provider = CopilotProvider::new(copilot_config.clone(), model)?;
                Ok(Arc::new(provider))
            }
            name => Err(ProviderError::Config(format!("Unknown provider: {}", name))),
        }
    }

    /// List all available provider names
    pub fn available_providers() -> Vec<&'static str> {
        vec!["ollama", "openai", "anthropic", "copilot"]
    }

    /// Check if a provider name is valid
    pub fn is_valid_provider(name: &str) -> bool {
        Self::available_providers().contains(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::providers::config::{OllamaConfig, OpenAiConfig};

    #[test]
    fn test_available_providers() {
        let providers = ProviderFactory::available_providers();
        assert_eq!(providers.len(), 4);
        assert!(providers.contains(&"ollama"));
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"anthropic"));
        assert!(providers.contains(&"copilot"));
    }

    #[test]
    fn test_is_valid_provider() {
        assert!(ProviderFactory::is_valid_provider("ollama"));
        assert!(ProviderFactory::is_valid_provider("openai"));
        assert!(ProviderFactory::is_valid_provider("anthropic"));
        assert!(ProviderFactory::is_valid_provider("copilot"));
        assert!(!ProviderFactory::is_valid_provider("unknown"));
    }

    #[test]
    fn test_create_ollama_provider() {
        let config = AiConfig {
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            fast_model: None,
            openai: None,
            anthropic: None,
            ollama: Some(OllamaConfig {
                base_url: "http://localhost:11434".to_string(),
                timeout: 300,
            }),
            copilot: None,
            retry: None,
        };

        let provider = ProviderFactory::create(&config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_create_openai_provider() {
        let config = AiConfig {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            fast_model: Some("gpt-4o-mini".to_string()),
            openai: Some(OpenAiConfig {
                api_key: "test-key".to_string(),
                base_url: "https://api.openai.com".to_string(),
                timeout: 600,
            }),
            anthropic: None,
            ollama: None,
            copilot: None,
            retry: None,
        };

        let provider = ProviderFactory::create(&config);
        assert!(provider.is_ok());

        let fast_provider = ProviderFactory::create_fast(&config);
        assert!(fast_provider.is_ok());
    }

    #[test]
    fn test_create_unknown_provider() {
        let config = AiConfig {
            provider: "unknown".to_string(),
            model: "test".to_string(),
            fast_model: None,
            openai: None,
            anthropic: None,
            ollama: None,
            copilot: None,
            retry: None,
        };

        let result = ProviderFactory::create(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::Config(_))));
    }

    #[test]
    fn test_create_missing_config() {
        let config = AiConfig {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            fast_model: None,
            openai: None, // Missing config
            anthropic: None,
            ollama: None,
            copilot: None,
            retry: None,
        };

        let result = ProviderFactory::create(&config);
        assert!(result.is_err());
        assert!(matches!(result, Err(ProviderError::Config(_))));
    }
}
