use crate::ai::providers::types::{Message, Role, Tool, ToolCall};
use serde::{Deserialize, Serialize};

/// OpenAI API request format
#[derive(Debug, Serialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    pub stream: bool,
}

/// OpenAI message format
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI tool definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiTool {
    pub r#type: String,
    pub function: OpenAiFunction,
}

/// OpenAI function definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// OpenAI tool call
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiToolCall {
    pub id: String,
    pub r#type: String,
    pub function: OpenAiFunctionCall,
}

/// OpenAI function call details
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAiFunctionCall {
    pub name: String,
    pub arguments: String,
}

impl From<Message> for OpenAiMessage {
    fn from(msg: Message) -> Self {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        }
        .to_string();

        let tool_calls = if msg.tool_calls.is_empty() {
            None
        } else {
            Some(msg.tool_calls.into_iter().map(Into::into).collect())
        };

        Self {
            role,
            content: msg.content,
            name: msg.name,
            tool_calls,
            tool_call_id: msg.tool_call_id,
        }
    }
}

impl From<Tool> for OpenAiTool {
    fn from(tool: Tool) -> Self {
        Self {
            r#type: "function".to_string(),
            function: OpenAiFunction {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
            },
        }
    }
}

impl From<ToolCall> for OpenAiToolCall {
    fn from(call: ToolCall) -> Self {
        Self {
            id: call.id,
            r#type: call.r#type,
            function: OpenAiFunctionCall {
                name: call.function.name,
                arguments: call.function.arguments,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::providers::types::FunctionCall;

    #[test]
    fn test_message_conversion() {
        let msg = Message::user("Hello");
        let openai_msg: OpenAiMessage = msg.into();
        assert_eq!(openai_msg.role, "user");
        assert_eq!(openai_msg.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_tool_call_conversion() {
        let call = ToolCall {
            id: "call_1".to_string(),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: "test".to_string(),
                arguments: "{}".to_string(),
            },
        };
        let openai_call: OpenAiToolCall = call.into();
        assert_eq!(openai_call.id, "call_1");
        assert_eq!(openai_call.function.name, "test");
    }
}
