//! where: iclaw/canister/src/service/compression/mod.rs
//! what: staged payload compaction and request-size estimation helpers
//! why: keep chars and bytes guards coherent while isolating compression responsibilities

mod compact;
mod llm_summary;
mod request;
#[cfg(test)]
mod tests;

use crate::provider::MAX_REQUEST_BYTES;
use crate::types::ContextConfig;
use iclaw_core::providers::ConversationMessage;
use iclaw_core::tools::ToolSpec;

const DEFAULT_MAX_PROMPT_CHARS: usize = 16_000;
const DEFAULT_MAX_REQUEST_BYTES_BUDGET: usize = MAX_REQUEST_BYTES - (16 * 1024);
const DEFAULT_LLM_SUMMARY_MODEL: &str = "gpt-4o-mini";
const DEFAULT_LLM_SUMMARY_MAX_CHARS: usize = 480;
const MIN_BODY_MESSAGES_TO_KEEP: usize = 3;
const MIN_BODY_MESSAGES_AFTER_LLM: usize = 1;
const COMPACTION_NOTICE: &str =
    "[Runtime context note]\nEarlier history was compacted. Ask for a refresh if older context is needed.";
const SESSION_SUMMARY_HEADER: &str = "[Session summary]";
const COMPACTED_SUMMARY_HEADER: &str = "[Compacted session summary]";

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

pub(crate) use compact::compact_messages;

pub(crate) fn estimate_messages_chars(messages: &[ConversationMessage]) -> usize {
    request::estimate_messages_chars(messages)
}

pub(crate) fn estimate_request_bytes(
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
) -> usize {
    request::estimate_request_bytes(messages, tools, model, temperature)
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
