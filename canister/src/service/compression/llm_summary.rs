//! where: iclaw/canister/src/service/compression/llm_summary.rs
//! what: overflow-only LLM summary helpers for compacted chat history
//! why: keep optional summary generation isolated from the main compaction loop

use crate::provider::IcCanisterProvider;
use iclaw_core::providers::{ChatMessage, ConversationMessage, ToolResultMessage};
use std::sync::Arc;

use super::{summary_body, truncate_chars};

const MAX_SUMMARY_SOURCE_CHARS: usize = 4_000;
const MAX_SUMMARY_SOURCE_MESSAGES: usize = 12;
const SUMMARY_SYSTEM_PROMPT: &str = "Summarize earlier chat context for continued use. Keep only durable facts, user preferences, unresolved tasks, and important tool outcomes. Return plain text only.";

pub(super) async fn generate_llm_summary(
    provider: &Arc<dyn IcCanisterProvider>,
    dropped: &[ConversationMessage],
    current_summary: Option<&ConversationMessage>,
    model: &str,
    max_chars: usize,
) -> Option<String> {
    let source = summary_source(dropped, current_summary);
    if source.trim().is_empty() {
        return None;
    }
    let request = vec![
        ConversationMessage::Chat(ChatMessage::system(SUMMARY_SYSTEM_PROMPT)),
        ConversationMessage::Chat(ChatMessage::user(source)),
    ];
    let response = provider.chat(&request, None, model, 0.0).await.ok()?;
    let text = response.response.text_or_empty().trim();
    if text.is_empty() {
        None
    } else {
        Some(truncate_chars(text, max_chars))
    }
}

fn summary_source(
    dropped: &[ConversationMessage],
    current_summary: Option<&ConversationMessage>,
) -> String {
    let mut lines = Vec::new();
    if let Some(ConversationMessage::Chat(chat)) = current_summary {
        let body = summary_body(&chat.content).trim();
        if !body.is_empty() {
            lines.push(format!(
                "Existing summary:\n{}",
                truncate_chars(body, MAX_SUMMARY_SOURCE_CHARS / 2)
            ));
        }
    }
    let dropped_lines = dropped
        .iter()
        .rev()
        .take(MAX_SUMMARY_SOURCE_MESSAGES)
        .rev()
        .map(render_summary_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if !dropped_lines.is_empty() {
        lines.push(format!(
            "Dropped earlier messages:\n{}",
            dropped_lines.join("\n")
        ));
    }
    truncate_chars(&lines.join("\n\n"), MAX_SUMMARY_SOURCE_CHARS)
}

fn render_summary_line(message: &ConversationMessage) -> String {
    match message {
        ConversationMessage::Chat(chat) => {
            format!(
                "{}: {}",
                chat.role,
                truncate_chars(chat.content.trim(), 320)
            )
        }
        ConversationMessage::AssistantToolCalls {
            text, tool_calls, ..
        } => format!(
            "assistant_tool_calls: {} {}",
            text.as_deref()
                .map(|value| truncate_chars(value.trim(), 160))
                .unwrap_or_default(),
            tool_calls
                .iter()
                .map(|call| format!(
                    "{}({})",
                    call.name,
                    truncate_chars(call.arguments.trim(), 120)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ConversationMessage::ToolResults(results) => format_tool_results(results),
    }
}

fn format_tool_results(results: &[ToolResultMessage]) -> String {
    format!(
        "tool_results: {}",
        results
            .iter()
            .map(|result| truncate_chars(result.content.trim(), 160))
            .collect::<Vec<_>>()
            .join(" | ")
    )
}
