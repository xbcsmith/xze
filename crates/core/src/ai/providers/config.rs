use serde::{Deserialize, Serialize};

/// Main AI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// The AI provider to use (ollama, openai, anthropic, copilot)
    pub provider: String,

    /// The default model to use
    pub model: String,

    /// Optional faster model for less complex tasks
    pub fast_model: Option<String>,

    /// OpenAI specific configuration
    #[serde(default)]
    pub openai: Option<OpenAiConfig>,

    /// Anthropic specific configuration
    #[serde(default)]
    pub anthropic: Option<AnthropicConfig>,

    /// Ollama specific configuration
    #[serde(default)]
    pub ollama: Option<OllamaConfig>,

    /// GitHub Copilot specific configuration
    #[serde(default)]
    pub copilot: Option<CopilotConfig>,

    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryConfig>,
}

/// OpenAI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// API key (can be env var reference)
    pub api_key: String,

    /// Base URL for API requests
    #[serde(default = "default_openai_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Anthropic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// API key (can be env var reference)
    pub api_key: String,

    /// Base URL for API requests
    #[serde(default = "default_anthropic_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Base URL for API requests
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// GitHub Copilot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotConfig {
    /// Custom config directory
    pub config_dir: Option<String>,

    /// OAuth token (optional, for CI/containers)
    pub oauth_token: Option<String>,

    /// Base URL for API requests
    #[serde(default = "default_copilot_base_url")]
    pub base_url: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial delay in milliseconds
    #[serde(default = "default_initial_delay_ms")]
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds
    #[serde(default = "default_max_delay_ms")]
    pub max_delay_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

fn default_openai_base_url() -> String {
    "https://api.openai.com".to_string()
}
fn default_anthropic_base_url() -> String {
    "https://api.anthropic.com".to_string()
}
fn default_ollama_base_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_copilot_base_url() -> String {
    "https://api.githubcopilot.com".to_string()
}
fn default_timeout() -> u64 {
    600
}
fn default_max_attempts() -> u32 {
    3
}
fn default_initial_delay_ms() -> u64 {
    1000
}
fn default_max_delay_ms() -> u64 {
    60000
}
fn default_backoff_multiplier() -> f64 {
    2.0
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "llama2".to_string(),
            fast_model: None,
            openai: None,
            anthropic: None,
            ollama: Some(OllamaConfig {
                base_url: default_ollama_base_url(),
                timeout: default_timeout(),
            }),
            copilot: None,
            retry: Some(RetryConfig::default()),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_delay_ms: default_initial_delay_ms(),
            max_delay_ms: default_max_delay_ms(),
            backoff_multiplier: default_backoff_multiplier(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AiConfig::default();
        assert_eq!(config.provider, "ollama");
        assert!(config.ollama.is_some());
        assert_eq!(config.ollama.unwrap().base_url, "http://localhost:11434");
    }

    #[test]
    fn test_deserialization() {
        let yaml = r#"
            provider: openai
            model: gpt-4
            openai:
                api_key: sk-test
        "#;
        let config: AiConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.openai.unwrap().api_key, "sk-test");
    }
}
