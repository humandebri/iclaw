//! where: iclaw/canister/src/service/compression.rs
//! what: payload estimation and staged compaction for ICP provider chat calls
//! why: keep provider-bound payloads bounded with chars and request-bytes guards

use crate::provider::{IcCanisterProvider, MAX_REQUEST_BYTES};
use crate::types::ContextConfig;
use iclaw_core::providers::{ChatMessage, ConversationMessage, ToolResultMessage};
use iclaw_core::tools::ToolSpec;
use serde_json::{Map, Value};
use std::sync::Arc;

const DEFAULT_MAX_PROMPT_CHARS: usize = 16_000;
const DEFAULT_MAX_REQUEST_BYTES_BUDGET: usize = MAX_REQUEST_BYTES - (16 * 1024);
const DEFAULT_LLM_SUMMARY_MODEL: &str = "gpt-4o-mini";
const DEFAULT_LLM_SUMMARY_MAX_CHARS: usize = 480;
const MAX_SUMMARY_SOURCE_CHARS: usize = 4_000;
const MAX_SUMMARY_SOURCE_MESSAGES: usize = 12;
const MIN_BODY_MESSAGES_TO_KEEP: usize = 3;
const MIN_BODY_MESSAGES_AFTER_LLM: usize = 1;
const COMPACTION_NOTICE: &str =
    "[Runtime context note]\nEarlier history was compacted. Ask for a refresh if older context is needed.";
const SESSION_SUMMARY_HEADER: &str = "[Session summary]";
const COMPACTED_SUMMARY_HEADER: &str = "[Compacted session summary]";
const SUMMARY_SYSTEM_PROMPT: &str = "Summarize earlier chat context for continued use. Keep only durable facts, user preferences, unresolved tasks, and important tool outcomes. Return plain text only.";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CompressionState {
    pub generated_summary: Option<String>,
    pub llm_summary_attempted: bool,
    pub llm_summary_succeeded: bool,
}

#[derive(Clone, Debug)]
struct CompressionConfig {
    max_prompt_chars: usize,
    max_request_bytes_budget: usize,
    llm_summary_on_overflow: bool,
    llm_summary_model: String,
    llm_summary_max_chars: usize,
    llm_summary_request_bytes_threshold: usize,
}

#[derive(Clone, Debug)]
struct MessageSections {
    system: Option<ConversationMessage>,
    memory_context: Option<ConversationMessage>,
    session_summary: Option<ConversationMessage>,
    body: Vec<ConversationMessage>,
}

impl From<Option<&ContextConfig>> for CompressionConfig {
    fn from(value: Option<&ContextConfig>) -> Self {
        let max_request_bytes_budget = value
            .and_then(|config| config.max_request_bytes_budget)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or(DEFAULT_MAX_REQUEST_BYTES_BUDGET)
            .min(MAX_REQUEST_BYTES.saturating_sub(1));
        let llm_summary_request_bytes_threshold = value
            .and_then(|config| config.llm_summary_request_bytes_threshold)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or(max_request_bytes_budget)
            .min(max_request_bytes_budget);
        Self {
            max_prompt_chars: value
                .and_then(|config| config.max_prompt_chars)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or(DEFAULT_MAX_PROMPT_CHARS),
            max_request_bytes_budget,
            llm_summary_on_overflow: value
                .and_then(|config| config.llm_summary_on_overflow)
                .unwrap_or(true),
            llm_summary_model: value
                .and_then(|config| config.llm_summary_model.clone())
                .filter(|candidate| !candidate.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_LLM_SUMMARY_MODEL.to_string()),
            llm_summary_max_chars: value
                .and_then(|config| config.llm_summary_max_chars)
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or(DEFAULT_LLM_SUMMARY_MAX_CHARS),
            llm_summary_request_bytes_threshold,
        }
    }
}

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
        && (estimate_sections_chars(&sections) > resolved.max_prompt_chars
            || estimate_sections_bytes(&sections, tools, model, temperature)
                > resolved.llm_summary_request_bytes_threshold)
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

pub(crate) fn estimate_messages_chars(messages: &[ConversationMessage]) -> usize {
    messages.iter().map(message_char_len).sum::<usize>() + messages.len().saturating_sub(1) * 2
}

pub(crate) fn estimate_request_bytes(
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
) -> usize {
    let mut request = Map::new();
    request.insert("model".to_string(), Value::String(model.to_string()));
    request.insert(
        "messages".to_string(),
        Value::Array(render_request_messages(messages)),
    );
    request.insert("temperature".to_string(), Value::from(temperature));
    if let Some(tool_specs) = tools.filter(|items| !items.is_empty()) {
        request.insert("tool_choice".to_string(), Value::String("auto".to_string()));
        request.insert(
            "tools".to_string(),
            Value::Array(render_request_tools(tool_specs)),
        );
    }
    serde_json::to_vec(&Value::Object(request))
        .map(|body| body.len())
        .unwrap_or(usize::MAX)
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

async fn generate_llm_summary(
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
                truncate_chars(body, max_chars_for_source())
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

fn max_chars_for_source() -> usize {
    MAX_SUMMARY_SOURCE_CHARS / 2
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

fn render_request_messages(messages: &[ConversationMessage]) -> Vec<Value> {
    let mut rendered = Vec::new();
    for message in messages {
        match message {
            ConversationMessage::Chat(chat) => {
                let mut object = Map::new();
                object.insert("role".to_string(), Value::String(chat.role.clone()));
                object.insert("content".to_string(), Value::String(chat.content.clone()));
                rendered.push(Value::Object(object));
            }
            ConversationMessage::AssistantToolCalls {
                text,
                tool_calls,
                reasoning_content,
            } => {
                let mut object = Map::new();
                object.insert("role".to_string(), Value::String("assistant".to_string()));
                if let Some(value) = text {
                    object.insert("content".to_string(), Value::String(value.clone()));
                }
                if let Some(value) = reasoning_content {
                    object.insert(
                        "reasoning_content".to_string(),
                        Value::String(value.clone()),
                    );
                }
                if !tool_calls.is_empty() {
                    object.insert(
                        "tool_calls".to_string(),
                        Value::Array(
                            tool_calls
                                .iter()
                                .map(|call| {
                                    let mut tool_object = Map::new();
                                    tool_object
                                        .insert("id".to_string(), Value::String(call.id.clone()));
                                    tool_object.insert(
                                        "type".to_string(),
                                        Value::String("function".to_string()),
                                    );
                                    let mut function_object = Map::new();
                                    function_object.insert(
                                        "name".to_string(),
                                        Value::String(call.name.clone()),
                                    );
                                    function_object.insert(
                                        "arguments".to_string(),
                                        Value::String(call.arguments.clone()),
                                    );
                                    tool_object.insert(
                                        "function".to_string(),
                                        Value::Object(function_object),
                                    );
                                    Value::Object(tool_object)
                                })
                                .collect(),
                        ),
                    );
                }
                rendered.push(Value::Object(object));
            }
            ConversationMessage::ToolResults(results) => {
                for result in results {
                    let mut object = Map::new();
                    object.insert("role".to_string(), Value::String("tool".to_string()));
                    object.insert("content".to_string(), Value::String(result.content.clone()));
                    object.insert(
                        "tool_call_id".to_string(),
                        Value::String(result.tool_call_id.clone()),
                    );
                    rendered.push(Value::Object(object));
                }
            }
        }
    }
    rendered
}

fn render_request_tools(tools: &[ToolSpec]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut object = Map::new();
            object.insert("type".to_string(), Value::String("function".to_string()));
            let mut function_object = Map::new();
            function_object.insert("name".to_string(), Value::String(tool.name.clone()));
            function_object.insert(
                "description".to_string(),
                Value::String(tool.description.clone()),
            );
            function_object.insert("parameters".to_string(), tool.parameters.clone());
            object.insert("function".to_string(), Value::Object(function_object));
            Value::Object(object)
        })
        .collect()
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

fn summary_body(content: &str) -> &str {
    content
        .strip_prefix(SESSION_SUMMARY_HEADER)
        .or_else(|| content.strip_prefix(COMPACTED_SUMMARY_HEADER))
        .unwrap_or(content)
        .trim()
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}
