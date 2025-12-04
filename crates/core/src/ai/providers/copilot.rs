use crate::ai::providers::{
    formats::openai::{OpenAiMessage, OpenAiRequest, OpenAiTool},
    ApiClient, AuthMethod, CompletionResponse, CopilotConfig, Message, ModelConfig, Provider,
    ProviderError, ProviderMetadata, Tool,
};
use async_trait::async_trait;

/// GitHub Copilot provider implementation
pub struct CopilotProvider {
    client: ApiClient,
    config: CopilotConfig,
    default_model: String,
}

impl CopilotProvider {
    /// Create a new Copilot provider
    pub fn new(config: CopilotConfig, default_model: String) -> Result<Self, ProviderError> {
        let client = ApiClient::new(config.timeout, None)?
            .with_header("editor-version", "vscode/1.85.0")? // Mimic VS Code
            .with_header("copilot-integration-id", "vscode-chat")?;

        Ok(Self {
            client,
            config,
            default_model,
        })
    }

    async fn complete_internal(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[Tool],
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let openai_messages: Vec<OpenAiMessage> =
            messages.iter().map(|m| m.clone().into()).collect();

        let openai_tools: Option<Vec<OpenAiTool>> = if tools.is_empty() {
            None
        } else {
            Some(tools.iter().map(|t| t.clone().into()).collect())
        };

        let request = OpenAiRequest {
            model: model.to_string(),
            messages: openai_messages,
            tools: openai_tools,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            top_p: config.top_p,
            frequency_penalty: config.frequency_penalty,
            presence_penalty: config.presence_penalty,
            stop: config.stop.clone(),
            stream: false,
        };

        // Copilot uses OAuth token
        let token = self
            .config
            .oauth_token
            .clone()
            .ok_or_else(|| ProviderError::Config("Missing Copilot OAuth token".into()))?;
        let auth = AuthMethod::Bearer(token);

        self.client.post(&url, &request, &auth).await
    }
}

#[async_trait]
impl Provider for CopilotProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[Tool],
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError> {
        self.complete_internal(&self.default_model, messages, tools, config)
            .await
    }

    async fn complete_fast(
        &self,
        messages: &[Message],
        tools: &[Tool],
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError> {
        // Copilot usually has one main model, or we can use the same
        self.complete_internal(&self.default_model, messages, tools, config)
            .await
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "copilot".to_string(),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false, // Copilot vision support varies
            max_tokens: 8192,       // Conservative estimate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_copilot_provider_complete() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1677652288,
                "model": "copilot-chat",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello from Copilot!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 10,
                    "total_tokens": 20
                }
            }"#,
            )
            .create_async()
            .await;

        let config = CopilotConfig {
            config_dir: None,
            oauth_token: Some("test-token".to_string()),
            base_url: server.url(),
            timeout: 10,
        };

        let provider = CopilotProvider::new(config, "copilot-chat".to_string()).unwrap();

        let messages = vec![Message::user("Hello")];
        let response = provider
            .complete(&messages, &[], &ModelConfig::default())
            .await
            .unwrap();

        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("Hello from Copilot!")
        );

        mock.assert_async().await;
    }
}
