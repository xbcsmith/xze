use crate::ai::providers::{
    formats::openai::{OpenAiMessage, OpenAiRequest, OpenAiTool},
    ApiClient, AuthMethod, CompletionResponse, Message, ModelConfig, OpenAiConfig, Provider,
    ProviderError, ProviderMetadata, Tool,
};
use async_trait::async_trait;

/// OpenAI provider implementation
pub struct OpenAiProvider {
    client: ApiClient,
    config: OpenAiConfig,
    default_model: String,
    fast_model: String,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(
        config: OpenAiConfig,
        default_model: String,
        fast_model: Option<String>,
    ) -> Result<Self, ProviderError> {
        let client = ApiClient::new(config.timeout, None)?;
        let fast_model = fast_model.unwrap_or_else(|| "gpt-4o-mini".to_string());

        Ok(Self {
            client,
            config,
            default_model,
            fast_model,
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

        let auth = AuthMethod::Bearer(self.config.api_key.clone());

        self.client.post(&url, &request, &auth).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
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
        self.complete_internal(&self.fast_model, messages, tools, config)
            .await
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "openai".to_string(),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true, // GPT-4o supports vision
            max_tokens: 128_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_openai_provider_complete() {
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
                "model": "gpt-4o",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello there!"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 9,
                    "completion_tokens": 12,
                    "total_tokens": 21
                }
            }"#,
            )
            .create_async()
            .await;

        let config = OpenAiConfig {
            api_key: "test-key".to_string(),
            base_url: server.url(),
            timeout: 10,
        };

        let provider = OpenAiProvider::new(config, "gpt-4o".to_string(), None).unwrap();

        let messages = vec![Message::user("Hello")];
        let response = provider
            .complete(&messages, &[], &ModelConfig::default())
            .await
            .unwrap();

        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("Hello there!")
        );
        assert_eq!(response.model, "gpt-4o");

        mock.assert_async().await;
    }
}
