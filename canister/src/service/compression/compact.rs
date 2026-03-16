//! where: iclaw/canister/src/service/compression/compact.rs
//! what: staged chars-and-bytes compaction for provider-bound chat payloads
//! why: keep overflow handling predictable while limiting LLM summary use to bytes risk

use crate::provider::IcCanisterProvider;
use crate::types::ContextConfig;
use iclaw_core::providers::{ChatMessage, ConversationMessage};
use iclaw_core::tools::ToolSpec;
use std::sync::Arc;

use super::llm_summary::generate_llm_summary;
use super::{
    estimate_messages_chars, estimate_request_bytes, summary_body, truncate_chars,
    CompressionConfig, CompressionState, MessageSections, COMPACTED_SUMMARY_HEADER,
    COMPACTION_NOTICE, MIN_BODY_MESSAGES_AFTER_LLM, MIN_BODY_MESSAGES_TO_KEEP,
    SESSION_SUMMARY_HEADER,
};

pub(crate) async fn compact_messages(
    provider: &Arc<dyn IcCanisterProvider>,
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
    config: Option<&ContextConfig>,
    state: &mut CompressionState,
) -> Vec<ConversationMessage> {
    let resolved = CompressionConfig::from(config);
    let mut sections = split_messages(messages);
    if sections_fit(&sections, tools, model, temperature, &resolved) {
        return messages.to_vec();
    }

    let mut compacted = false;
    let mut dropped = trim_oldest_body_messages(
        &mut sections,
        tools,
        model,
        temperature,
        &resolved,
        MIN_BODY_MESSAGES_TO_KEEP,
    );
    compacted |= !dropped.is_empty();

    if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
        if let Some(summary) = sections.session_summary.take() {
            if let Some(compacted_summary) =
                compact_summary_message(&summary, resolved.llm_summary_max_chars)
            {
                sections.session_summary = Some(compacted_summary);
                compacted = true;
            }
        }
    }

    if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
        sections.memory_context = trim_optional_message(
            sections.memory_context.take(),
            resolved.max_prompt_chars / 6,
        );
        compacted = true;
    }

    dropped.extend(trim_oldest_body_messages(
        &mut sections,
        tools,
        model,
        temperature,
        &resolved,
        MIN_BODY_MESSAGES_AFTER_LLM,
    ));

    if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
        sections.session_summary = trim_optional_message(
            sections.session_summary.take(),
            resolved.max_prompt_chars / 10,
        );
    }
    if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
        sections.memory_context = trim_optional_message(
            sections.memory_context.take(),
            resolved.max_prompt_chars / 8,
        );
    }

    if sections_exceed_limits(&sections, tools, model, temperature, &resolved)
        && resolved.llm_summary_on_overflow
        && !state.llm_summary_attempted
        && estimate_sections_bytes(&sections, tools, model, temperature)
            > resolved.llm_summary_request_bytes_threshold
    {
        state.llm_summary_attempted = true;
        if let Some(summary) = generate_llm_summary(
            provider,
            &dropped,
            sections.session_summary.as_ref(),
            &resolved.llm_summary_model,
            resolved.llm_summary_max_chars,
        )
        .await
        {
            state.generated_summary = Some(summary.clone());
            state.llm_summary_succeeded = true;
            sections.session_summary = Some(ConversationMessage::Chat(ChatMessage::user(format!(
                "{COMPACTED_SUMMARY_HEADER}\n{summary}"
            ))));
            compacted = true;
        }
    }

    if compacted {
        append_compaction_notice(&mut sections.system);
        dropped.extend(trim_oldest_body_messages(
            &mut sections,
            tools,
            model,
            temperature,
            &resolved,
            MIN_BODY_MESSAGES_AFTER_LLM,
        ));
        if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
            sections.session_summary = trim_optional_message(
                sections.session_summary.take(),
                resolved.max_prompt_chars / 10,
            );
        }
        if sections_exceed_limits(&sections, tools, model, temperature, &resolved) {
            sections.memory_context = trim_optional_message(
                sections.memory_context.take(),
                resolved.max_prompt_chars / 8,
            );
        }
    }

    rebuild_messages(sections)
}

fn split_messages(messages: &[ConversationMessage]) -> MessageSections {
    let mut cursor = 0;
    let system = messages
        .first()
        .filter(
            |message| matches!(message, ConversationMessage::Chat(chat) if chat.role == "system"),
        )
        .cloned();
    if system.is_some() {
        cursor += 1;
    }
    let memory_context = messages
        .get(cursor)
        .filter(|message| is_tagged_user_message(message, "[Memory context]"))
        .cloned();
    if memory_context.is_some() {
        cursor += 1;
    }
    let session_summary = messages
        .get(cursor)
        .filter(|message| is_summary_message(message))
        .cloned();
    if session_summary.is_some() {
        cursor += 1;
    }

    MessageSections {
        system,
        memory_context,
        session_summary,
        body: messages[cursor..].to_vec(),
    }
}

fn rebuild_messages(sections: MessageSections) -> Vec<ConversationMessage> {
    let mut messages = Vec::new();
    if let Some(system) = sections.system {
        messages.push(system);
    }
    if let Some(memory_context) = sections.memory_context {
        messages.push(memory_context);
    }
    if let Some(session_summary) = sections.session_summary {
        messages.push(session_summary);
    }
    messages.extend(sections.body);
    messages
}

fn estimate_sections_chars(sections: &MessageSections) -> usize {
    estimate_messages_chars(&rebuild_messages(sections.clone()))
}

fn estimate_sections_bytes(
    sections: &MessageSections,
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
) -> usize {
    estimate_request_bytes(
        &rebuild_messages(sections.clone()),
        tools,
        model,
        temperature,
    )
}

fn sections_exceed_limits(
    sections: &MessageSections,
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
    config: &CompressionConfig,
) -> bool {
    estimate_sections_chars(sections) > config.max_prompt_chars
        || estimate_sections_bytes(sections, tools, model, temperature)
            > config.max_request_bytes_budget
}

fn sections_fit(
    sections: &MessageSections,
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
    config: &CompressionConfig,
) -> bool {
    !sections_exceed_limits(sections, tools, model, temperature, config)
}

fn trim_oldest_body_messages(
    sections: &mut MessageSections,
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
    config: &CompressionConfig,
    min_body_messages: usize,
) -> Vec<ConversationMessage> {
    let mut dropped = Vec::new();
    while sections_exceed_limits(sections, tools, model, temperature, config)
        && sections.body.len() > min_body_messages
    {
        dropped.push(sections.body.remove(0));
    }
    dropped
}

fn trim_optional_message(
    message: Option<ConversationMessage>,
    max_chars: usize,
) -> Option<ConversationMessage> {
    let message = message?;
    match message {
        ConversationMessage::Chat(chat) => {
            let trimmed = truncate_chars(&chat.content, max_chars);
            if trimmed.trim().is_empty() {
                None
            } else {
                Some(ConversationMessage::Chat(ChatMessage {
                    role: chat.role,
                    content: trimmed,
                }))
            }
        }
        other => Some(other),
    }
}

fn compact_summary_message(
    message: &ConversationMessage,
    max_chars: usize,
) -> Option<ConversationMessage> {
    match message {
        ConversationMessage::Chat(chat) => {
            let header = if chat.content.starts_with(COMPACTED_SUMMARY_HEADER) {
                COMPACTED_SUMMARY_HEADER
            } else {
                SESSION_SUMMARY_HEADER
            };
            let body = summary_body(&chat.content);
            let available = max_chars.saturating_sub(header.chars().count() + 1);
            if available == 0 {
                return None;
            }
            let trimmed = truncate_chars(body, available);
            if trimmed.trim().is_empty() {
                None
            } else {
                Some(ConversationMessage::Chat(ChatMessage::user(format!(
                    "{header}\n{trimmed}"
                ))))
            }
        }
        _ => None,
    }
}

fn append_compaction_notice(system: &mut Option<ConversationMessage>) {
    let Some(ConversationMessage::Chat(chat)) = system else {
        return;
    };
    if chat.content.contains(COMPACTION_NOTICE) {
        return;
    }
    chat.content = format!("{}\n\n{}", chat.content, COMPACTION_NOTICE);
}

fn is_tagged_user_message(message: &ConversationMessage, tag: &str) -> bool {
    matches!(message, ConversationMessage::Chat(chat) if chat.role == "user" && chat.content.starts_with(tag))
}

fn is_summary_message(message: &ConversationMessage) -> bool {
    is_tagged_user_message(message, SESSION_SUMMARY_HEADER)
        || is_tagged_user_message(message, COMPACTED_SUMMARY_HEADER)
}
