//! where: iclaw/canister/src/provider/messages.rs
//! what: OpenAI-compatible request/response DTOs and conversion helpers for the ICP provider
//! why: keep provider.rs focused on transport orchestration while isolating tool-call wire shape

use iclaw_core::providers::{ChatMessage, ChatResponse, ConversationMessage, ToolCall};
use iclaw_core::tools::ToolSpec;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<OpenAiMessage>,
    pub(crate) temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<Vec<OpenAiToolSpec>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiMessage {
    pub(crate) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct OpenAiToolCall {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(rename = "type", default)]
    pub(crate) kind: Option<String>,
    pub(crate) function: OpenAiFunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct OpenAiFunctionCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiToolSpec {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: OpenAiToolFunctionSpec,
}

#[derive(Debug, Serialize)]
pub(crate) struct OpenAiToolFunctionSpec {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChatResponse {
    pub(crate) model: Option<String>,
    pub(crate) choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiChoice {
    pub(crate) message: OpenAiResponseMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAiResponseMessage {
    #[serde(default)]
    pub(crate) content: Option<String>,
    #[serde(default)]
    pub(crate) reasoning_content: Option<String>,
    #[serde(default)]
    pub(crate) tool_calls: Option<Vec<OpenAiToolCall>>,
}

impl OpenAiResponseMessage {
    pub(crate) fn effective_content(&self) -> Option<String> {
        match &self.content {
            Some(content) if !content.is_empty() => Some(content.clone()),
            _ => self.reasoning_content.clone(),
        }
    }
}

pub(crate) fn convert_tools(tools: Option<&[ToolSpec]>) -> Option<Vec<OpenAiToolSpec>> {
    tools.map(|items| {
        items
            .iter()
            .map(|tool| OpenAiToolSpec {
                kind: "function".to_string(),
                function: OpenAiToolFunctionSpec {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.parameters.clone(),
                },
            })
            .collect()
    })
}

pub(crate) fn convert_messages(messages: &[ConversationMessage]) -> Vec<OpenAiMessage> {
    messages
        .iter()
        .flat_map(|message| match message {
            ConversationMessage::Chat(chat) => vec![OpenAiMessage {
                role: chat.role.clone(),
                content: Some(chat.content.clone()),
                tool_call_id: None,
                tool_calls: None,
                reasoning_content: None,
            }],
            ConversationMessage::AssistantToolCalls {
                text,
                tool_calls,
                reasoning_content,
            } => vec![OpenAiMessage {
                role: "assistant".to_string(),
                content: text.clone(),
                tool_call_id: None,
                tool_calls: Some(
                    tool_calls
                        .iter()
                        .map(|call| OpenAiToolCall {
                            id: Some(call.id.clone()),
                            kind: Some("function".to_string()),
                            function: OpenAiFunctionCall {
                                name: call.name.clone(),
                                arguments: call.arguments.clone(),
                            },
                        })
                        .collect(),
                ),
                reasoning_content: reasoning_content.clone(),
            }],
            ConversationMessage::ToolResults(results) => results
                .iter()
                .map(|result| OpenAiMessage {
                    role: "tool".to_string(),
                    content: Some(result.content.clone()),
                    tool_call_id: Some(result.tool_call_id.clone()),
                    tool_calls: None,
                    reasoning_content: None,
                })
                .collect(),
        })
        .collect()
}

pub(crate) fn parse_chat_response(message: OpenAiResponseMessage) -> ChatResponse {
    ChatResponse {
        text: message.effective_content(),
        tool_calls: message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|call| ToolCall {
                id: call
                    .id
                    .unwrap_or_else(|| "tool-call-missing-id".to_string()),
                name: call.function.name,
                arguments: call.function.arguments,
            })
            .collect(),
        usage: None,
        reasoning_content: message.reasoning_content,
    }
}

#[allow(dead_code)]
pub(crate) fn simple_messages(
    system_prompt: Option<&str>,
    prompt: &str,
) -> Vec<ConversationMessage> {
    let mut messages = Vec::new();
    if let Some(system_prompt) = system_prompt {
        messages.push(ConversationMessage::Chat(ChatMessage::system(
            system_prompt,
        )));
    }
    messages.push(ConversationMessage::Chat(ChatMessage::user(prompt)));
    messages
}
