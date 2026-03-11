//! where: iclaw/canister/src/service/compression/request.rs
//! what: request-shape helpers for chars and exact bytes estimation
//! why: keep compression preflight aligned with the provider's actual JSON body

use crate::provider::messages::build_chat_request;
use iclaw_core::providers::ConversationMessage;
use iclaw_core::tools::ToolSpec;

pub(crate) fn estimate_messages_chars(messages: &[ConversationMessage]) -> usize {
    messages.iter().map(message_char_len).sum::<usize>() + messages.len().saturating_sub(1) * 2
}

pub(crate) fn estimate_request_bytes(
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
) -> usize {
    serde_json::to_vec(&build_chat_request(messages, tools, model, temperature))
        .map(|body| body.len())
        .unwrap_or(usize::MAX)
}

fn message_char_len(message: &ConversationMessage) -> usize {
    match message {
        ConversationMessage::Chat(chat) => chat.role.chars().count() + chat.content.chars().count(),
        ConversationMessage::AssistantToolCalls {
            text,
            tool_calls,
            reasoning_content,
        } => {
            text.as_deref()
                .map(str::chars)
                .map(Iterator::count)
                .unwrap_or(0)
                + reasoning_content
                    .as_deref()
                    .map(str::chars)
                    .map(Iterator::count)
                    .unwrap_or(0)
                + tool_calls
                    .iter()
                    .map(|call| {
                        call.id.chars().count()
                            + call.name.chars().count()
                            + call.arguments.chars().count()
                    })
                    .sum::<usize>()
        }
        ConversationMessage::ToolResults(results) => results
            .iter()
            .map(|result| result.tool_call_id.chars().count() + result.content.chars().count())
            .sum(),
    }
}
