pub mod anthropic;
pub mod api_client;
pub mod base;
pub mod config;
pub mod copilot;
pub mod errors;
pub mod factory;
pub mod formats;
pub mod ollama;
pub mod openai;
pub mod retry;
pub mod types;

// Re-export key types
pub use api_client::{ApiClient, AuthMethod};
pub use base::{Provider, ProviderMetadata};
pub use config::{
    AiConfig, AnthropicConfig, CopilotConfig, OllamaConfig, OpenAiConfig, RetryConfig,
};
pub use errors::ProviderError;
pub use factory::ProviderFactory;
pub use retry::RetryStrategy;
pub use types::{
    Choice, CompletionResponse, FunctionCall, Message, ModelConfig, Role, Tool, ToolCall, Usage,
};

// Re-export provider implementations
pub use anthropic::AnthropicProvider;
pub use copilot::CopilotProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
