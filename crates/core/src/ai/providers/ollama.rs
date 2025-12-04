use crate::ai::providers::{
    types::{Choice, Role, Usage},
    ApiClient, AuthMethod, CompletionResponse, Message, ModelConfig, OllamaConfig, Provider,
    ProviderError, ProviderMetadata, Tool,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Ollama provider implementation
pub struct OllamaProvider {
    client: ApiClient,
    config: OllamaConfig,
    default_model: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: OllamaConfig, default_model: String) -> Result<Self, ProviderError> {
        let client = ApiClient::new(config.timeout, None)?;
        Ok(Self {
            client,
            config,
            default_model,
        })
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    async fn complete(
        &self,
        messages: &[Message],
        _tools: &[Tool], // Ollama tool support is experimental/varying, skipping for now or need specific implementation
        config: &ModelConfig,
    ) -> Result<CompletionResponse, ProviderError> {
        let url = format!("{}/api/chat", self.config.base_url);

        let request = OllamaChatRequest {
            model: self.default_model.clone(),
            messages: messages.iter().map(OllamaMessage::from).collect(),
            stream: false,
            options: Some(OllamaOptions::from(config)),
        };

        let response: OllamaChatResponse =
            self.client.post(&url, &request, &AuthMethod::None).await?;

        Ok(CompletionResponse {
            id: "ollama-chat".to_string(), // Ollama doesn't return an ID
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64, // Approximate
            model: response.model,
            choices: vec![Choice {
                index: 0,
                message: response.message.into(),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: response.prompt_eval_count.unwrap_or(0) as u32,
                completion_tokens: response.eval_count.unwrap_or(0) as u32,
                total_tokens: (response.prompt_eval_count.unwrap_or(0)
                    + response.eval_count.unwrap_or(0)) as u32,
            }),
        })
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "ollama".to_string(),
            supports_streaming: false, // Not implemented yet in this provider
            supports_tools: false,     // Explicitly disabled for now
            supports_vision: false,
            max_tokens: 4096, // Default assumption
        }
    }
}

// --- Ollama API Types ---

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    #[serde(default)]
    prompt_eval_count: Option<i32>,
    #[serde(default)]
    eval_count: Option<i32>,
}

// --- Conversions ---

impl From<&Message> for OllamaMessage {
    fn from(msg: &Message) -> Self {
        Self {
            role: match msg.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::Tool => "tool".to_string(), // Ollama might not support this yet
            },
            content: msg.content.clone().unwrap_or_default(),
            images: None,
        }
    }
}

impl From<OllamaMessage> for Message {
    fn from(msg: OllamaMessage) -> Self {
        let role = match msg.role.as_str() {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "tool" => Role::Tool,
            _ => Role::User, // Fallback
        };

        Message {
            role,
            content: Some(msg.content),
            name: None,
            tool_calls: vec![],
            tool_call_id: None,
        }
    }
}

impl From<&ModelConfig> for OllamaOptions {
    fn from(config: &ModelConfig) -> Self {
        Self {
            temperature: config.temperature,
            num_predict: config.max_tokens.map(|t| t as i32),
            top_p: config.top_p,
            stop: config.stop.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_ollama_provider_complete() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "model": "llama2",
                "created_at": "2023-08-04T19:22:45.499127Z",
                "message": {
                    "role": "assistant",
                    "content": "The sky is blue."
                },
                "done": true,
                "prompt_eval_count": 10,
                "eval_count": 5
            }"#,
            )
            .create_async()
            .await;

        let config = OllamaConfig {
            base_url: server.url(),
            timeout: 10,
        };

        let provider = OllamaProvider::new(config, "llama2".to_string()).unwrap();

        let messages = vec![Message::user("Why is the sky blue?")];
        let response = provider
            .complete(&messages, &[], &ModelConfig::default())
            .await
            .unwrap();

        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("The sky is blue.")
        );
        assert_eq!(response.usage.unwrap().total_tokens, 15);

        mock.assert_async().await;
    }
}
