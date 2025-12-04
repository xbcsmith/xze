use crate::ai::providers::{
    formats::anthropic::{AnthropicContent, AnthropicMessage, AnthropicRequest, AnthropicTool},
    types::{Choice, FunctionCall, Role, ToolCall, Usage},
    AnthropicConfig, ApiClient, AuthMethod, CompletionResponse, Message, ModelConfig, Provider,
    ProviderError, ProviderMetadata, Tool,
};
use async_trait::async_trait;
use serde::Deserialize;

/// Anthropic provider implementation
pub struct AnthropicProvider {
    client: ApiClient,
    config: AnthropicConfig,
    default_model: String,
    fast_model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider
    pub fn new(
        config: AnthropicConfig,
        default_model: String,
        fast_model: Option<String>,
    ) -> Result<Self, ProviderError> {
        let client =
            ApiClient::new(config.timeout, None)?.with_header("anthropic-version", "2023-06-01")?;

        let fast_model = fast_model.unwrap_or_else(|| "claude-3-haiku-20240307".to_string());

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
        let url = format!("{}/messages", self.config.base_url.trim_end_matches('/'));

        // Extract system prompt
        let mut system_prompt = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    if let Some(content) = &msg.content {
                        if system_prompt.is_some() {
                            // Append to existing system prompt
                            let existing = system_prompt.take().unwrap();
                            system_prompt = Some(format!("{}\n\n{}", existing, content));
                        } else {
                            system_prompt = Some(content.clone());
                        }
                    }
                }
                Role::User | Role::Assistant => {
                    let content = if let Some(text) = &msg.content {
                        vec![AnthropicContent::Text { text: text.clone() }]
                    } else if !msg.tool_calls.is_empty() {
                        msg.tool_calls
                            .iter()
                            .map(|tc| AnthropicContent::ToolUse {
                                id: tc.id.clone(),
                                name: tc.function.name.clone(),
                                input: serde_json::from_str(&tc.function.arguments)
                                    .unwrap_or_default(),
                            })
                            .collect()
                    } else {
                        vec![]
                    };

                    if !content.is_empty() {
                        anthropic_messages.push(AnthropicMessage {
                            role: if msg.role == Role::User {
                                "user".to_string()
                            } else {
                                "assistant".to_string()
                            },
                            content,
                        });
                    }
                }
                Role::Tool => {
                    // Tool results are user messages in Anthropic
                    let content = vec![AnthropicContent::ToolResult {
                        tool_use_id: msg.tool_call_id.clone().unwrap_or_default(),
                        content: msg.content.clone().unwrap_or_default(),
                        is_error: None,
                    }];

                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content,
                    });
                }
            }
        }

        let anthropic_tools: Option<Vec<AnthropicTool>> = if tools.is_empty() {
            None
        } else {
            Some(tools.iter().map(|t| t.clone().into()).collect())
        };

        let request = AnthropicRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            system: system_prompt,
            max_tokens: config.max_tokens.unwrap_or(4096),
            tools: anthropic_tools,
            temperature: config.temperature,
            top_p: config.top_p,
            stop_sequences: config.stop.clone(),
            stream: false,
        };

        let auth = AuthMethod::ApiKey {
            key: self.config.api_key.clone(),
            header: "x-api-key".to_string(),
        };

        // Anthropic requires this header
        // We need to add it manually or update ApiClient to support extra headers
        // For now, let's assume ApiClient handles standard auth and we might need to extend it
        // Or we can use a custom AuthMethod if we modify ApiClient, but ApiClient is generic.
        // Actually, ApiClient supports custom headers via AuthMethod::ApiKey, but Anthropic needs TWO headers:
        // x-api-key AND anthropic-version.
        // The current ApiClient implementation doesn't support multiple auth headers easily.
        // I should probably update ApiClient to allow custom headers per request or just hack it here.
        // But `post` takes `AuthMethod`.
        // Let's rely on `ApiClient` for `x-api-key` and maybe I need to add `anthropic-version` to `ApiClient` or `AuthMethod`.
        // Or I can just add it to the request if I had access to headers.
        // `ApiClient::post` doesn't expose headers.

        // FIX: I will update ApiClient to support custom headers in `post` or add `anthropic-version` to `AuthMethod`?
        // No, `AuthMethod` is for auth. `anthropic-version` is versioning.
        // I should update `ApiClient` to accept extra headers.
        // But for now, I will assume `ApiClient` is what I wrote.
        // I will update `ApiClient` in a separate step if needed.
        // For now, I'll use `AuthMethod::ApiKey` for `x-api-key`.
        // And I'll miss `anthropic-version` which might cause errors.
        // Wait, `anthropic-version: 2023-06-01` is REQUIRED.

        // I will modify `ApiClient` to support default headers or extra headers.
        // But I can't modify `ApiClient` right now inside this file write.
        // I will write this file assuming I can fix `ApiClient` later or use a workaround.
        // Workaround: I can't easily workaround without changing `ApiClient`.
        // I will add a TODO comment.

        let response: AnthropicResponse = self.client.post(&url, &request, &auth).await?;

        Ok(response.into())
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
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
            name: "anthropic".to_string(),
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            max_tokens: 200_000,
        }
    }
}

// --- Response Types ---

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[allow(dead_code)]
    r#type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

impl From<AnthropicResponse> for CompletionResponse {
    fn from(resp: AnthropicResponse) -> Self {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in resp.content {
            match block {
                AnthropicContent::Text { text } => {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(&text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
                _ => {}
            }
        }

        CompletionResponse {
            id: resp.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp() as u64,
            model: resp.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: match resp.role.as_str() {
                        "assistant" => Role::Assistant,
                        _ => Role::Assistant,
                    },
                    content: if content.is_empty() && !tool_calls.is_empty() {
                        None
                    } else {
                        Some(content)
                    },
                    name: None,
                    tool_calls,
                    tool_call_id: None,
                },
                finish_reason: resp.stop_reason,
            }],
            usage: Some(Usage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_anthropic_provider_complete() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/messages")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "content": [
                    {
                        "type": "text",
                        "text": "Hello!"
                    }
                ],
                "model": "claude-3-opus",
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 5
                }
            }"#,
            )
            .create_async()
            .await;

        let config = AnthropicConfig {
            api_key: "test-key".to_string(),
            base_url: server.url(),
            timeout: 10,
        };

        let provider = AnthropicProvider::new(config, "claude-3-opus".to_string(), None).unwrap();

        let messages = vec![Message::user("Hello")];
        let response = provider
            .complete(&messages, &[], &ModelConfig::default())
            .await
            .unwrap();

        assert_eq!(
            response.choices[0].message.content.as_deref(),
            Some("Hello!")
        );
        assert_eq!(response.model, "claude-3-opus");

        mock.assert_async().await;
    }
}
