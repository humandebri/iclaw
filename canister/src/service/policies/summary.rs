//! where: iclaw/canister/src/service/policies/summary.rs
//! what: deterministic rolling summary builder for ICP chat sessions
//! why: preserve long-running context without adding timers or extra LLM summary calls

use super::{promoted_summary_line, PromotionCandidate, PromotionCategory};
use crate::context::{enable_conversation_summary, history_limit, summary_max_chars};
use crate::types::ContextConfig;
use iclaw_core::memory::{Memory, MemoryCategory};
use iclaw_core::providers::{ChatMessage, ConversationMessage};
use std::collections::BTreeSet;
use std::sync::Arc;

use super::super::agent::sanitize_session_key;

const SUMMARY_REFRESH_INTERVAL: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
struct SummaryState {
    turn_count: usize,
    summary_turn_count: usize,
    content: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SummarySections {
    preferences: Vec<String>,
    facts: Vec<String>,
    open_threads: Vec<String>,
    recent_tools: Vec<String>,
}

pub(crate) async fn load_conversation_summary(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<String> {
    if !enable_conversation_summary(config) {
        return Ok(String::new());
    }
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(String::new());
    };
    let Some(entry) = memory.get(&summary_key(session_id)).await? else {
        return Ok(String::new());
    };
    Ok(parse_summary_state(&entry.content)
        .map(|state| state.content)
        .filter(|content| !content.is_empty())
        .unwrap_or_default())
}

pub(crate) async fn record_conversation_turn(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
) -> anyhow::Result<()> {
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(());
    };
    let key = summary_key(session_id);
    let next_state = match memory
        .get(&key)
        .await?
        .and_then(|entry| parse_summary_state(&entry.content))
    {
        Some(state) => SummaryState {
            turn_count: state.turn_count + 1,
            ..state
        },
        None => SummaryState {
            turn_count: 1,
            summary_turn_count: 0,
            content: String::new(),
        },
    };
    memory
        .store(
            &key,
            &render_summary_state(&next_state),
            MemoryCategory::Conversation,
            Some(session_id),
        )
        .await
}

pub(crate) async fn refresh_conversation_summary(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    history: &[ConversationMessage],
    promoted: Option<&PromotionCandidate>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<()> {
    if !enable_conversation_summary(config) {
        return Ok(());
    }
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(());
    };
    let existing = memory
        .get(&summary_key(session_id))
        .await?
        .and_then(|entry| parse_summary_state(&entry.content));
    let turn_count = existing
        .as_ref()
        .map_or(history.len(), |state| state.turn_count);
    if turn_count < history_limit(config) {
        return Ok(());
    }
    if existing
        .as_ref()
        .is_some_and(|state| turn_count < state.summary_turn_count + SUMMARY_REFRESH_INTERVAL)
    {
        return Ok(());
    }

    let summary = build_summary(
        history,
        existing.as_ref().map(|state| state.content.as_str()),
        promoted,
        summary_max_chars(config),
    );
    if summary.is_empty() {
        return Ok(());
    }
    let stored = render_summary_state(&SummaryState {
        turn_count,
        summary_turn_count: turn_count,
        content: summary,
    });
    memory
        .store(
            &summary_key(session_id),
            &stored,
            MemoryCategory::Conversation,
            Some(session_id),
        )
        .await
}

pub(crate) async fn replace_conversation_summary(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    turn_count_hint: usize,
    content: &str,
) -> anyhow::Result<()> {
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(());
    };
    if content.trim().is_empty() {
        return Ok(());
    }
    let key = summary_key(session_id);
    let turn_count = memory
        .get(&key)
        .await?
        .and_then(|entry| parse_summary_state(&entry.content))
        .map(|state| state.turn_count.max(turn_count_hint))
        .unwrap_or(turn_count_hint);
    let stored = render_preserved_summary_state(&SummaryState {
        turn_count,
        summary_turn_count: turn_count,
        content: format!("[Compacted session summary]\n{}", content.trim()),
    });
    memory
        .store(
            &key,
            &stored,
            MemoryCategory::Conversation,
            Some(session_id),
        )
        .await
}

pub(crate) fn is_summary_key(key: &str) -> bool {
    key.starts_with("conversation_summary/")
}

pub(crate) fn summary_key(session_id: &str) -> String {
    format!("conversation_summary/{}", sanitize_session_key(session_id))
}

#[cfg(test)]
#[allow(dead_code)]
pub(super) fn parse_summary_turn_count(content: &str) -> Option<usize> {
    parse_summary_state(content).map(|state| state.turn_count)
}

fn parse_summary_state(content: &str) -> Option<SummaryState> {
    let mut lines = content.lines();
    let first = lines.next()?.trim();
    let turn_count = first.strip_prefix("turn_count:")?.parse().ok()?;
    let second = lines.next()?.trim();
    let summary_turn_count = second.strip_prefix("summary_turn_count:")?.parse().ok()?;
    let remaining = lines.collect::<Vec<_>>();
    let summary = remaining.join("\n").trim().to_string();
    Some(SummaryState {
        turn_count,
        summary_turn_count,
        content: summary,
    })
}

fn render_summary_state(state: &SummaryState) -> String {
    let normalized = normalize_summary_content(&state.content);
    if state.content.is_empty() {
        return format!(
            "turn_count:{}\nsummary_turn_count:{}",
            state.turn_count, state.summary_turn_count
        );
    }
    format!(
        "turn_count:{}\nsummary_turn_count:{}\n{}",
        state.turn_count, state.summary_turn_count, normalized
    )
}

fn render_preserved_summary_state(state: &SummaryState) -> String {
    if state.content.is_empty() {
        return format!(
            "turn_count:{}\nsummary_turn_count:{}",
            state.turn_count, state.summary_turn_count
        );
    }
    format!(
        "turn_count:{}\nsummary_turn_count:{}\n{}",
        state.turn_count, state.summary_turn_count, state.content
    )
}

fn normalize_summary_content(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let body = trimmed
        .strip_prefix("[Session summary]")
        .or_else(|| trimmed.strip_prefix("[Compacted session summary]"))
        .map(str::trim)
        .unwrap_or(trimmed);
    format!("[Session summary]\n{}", body)
}

pub(super) fn build_summary(
    history: &[ConversationMessage],
    existing_summary: Option<&str>,
    promoted: Option<&PromotionCandidate>,
    max_chars: usize,
) -> String {
    let mut sections = parse_summary_sections(existing_summary.unwrap_or_default());
    merge_history(&mut sections, history);
    merge_promoted(&mut sections, promoted);
    render_sections(&sections, max_chars)
}

fn merge_history(sections: &mut SummarySections, history: &[ConversationMessage]) {
    for message in history.iter().rev().take(8) {
        match message {
            ConversationMessage::Chat(ChatMessage { role, content, .. }) => {
                let Some(line) = super::promote::first_important_sentence(content) else {
                    continue;
                };
                if role == "user" {
                    push_unique_front(&mut sections.open_threads, format!("- {}", line), 4);
                } else if role == "assistant" {
                    push_unique_front(&mut sections.facts, format!("- {}", line), 4);
                }
            }
            ConversationMessage::AssistantToolCalls {
                text, tool_calls, ..
            } => {
                if let Some(text) = text
                    .as_deref()
                    .and_then(super::promote::first_important_sentence)
                {
                    push_unique_front(&mut sections.open_threads, format!("- {}", text), 4);
                }
                for tool_call in tool_calls.iter().rev().take(2) {
                    push_unique_front(
                        &mut sections.recent_tools,
                        format!("- requested {}", tool_call.name),
                        4,
                    );
                }
            }
            ConversationMessage::ToolResults(results) => {
                for result in results.iter().rev().take(2) {
                    let status = if result.content.contains("\"success\":true") {
                        "succeeded"
                    } else {
                        "failed"
                    };
                    push_unique_front(
                        &mut sections.recent_tools,
                        format!("- tool {} {}", result.tool_call_id, status),
                        4,
                    );
                }
            }
        }
    }
}

fn merge_promoted(sections: &mut SummarySections, promoted: Option<&PromotionCandidate>) {
    let Some(promoted) = promoted else {
        return;
    };
    let line = promoted_summary_line(promoted);
    match promoted.category {
        PromotionCategory::Preference => push_unique_front(&mut sections.preferences, line, 4),
        PromotionCategory::Fact => push_unique_front(&mut sections.facts, line, 4),
        PromotionCategory::Goal => push_unique_front(&mut sections.open_threads, line, 4),
    }
}

fn parse_summary_sections(content: &str) -> SummarySections {
    let mut sections = SummarySections::default();
    let mut current = "";
    for line in content.lines().map(str::trim) {
        match line {
            "[Session summary]" | "[Compacted session summary]" | "" => continue,
            "Preferences:" => current = "preferences",
            "Facts:" => current = "facts",
            "Open threads:" => current = "open_threads",
            "Recent tools:" => current = "recent_tools",
            _ if line.starts_with("- ") => match current {
                "preferences" => push_unique_back(&mut sections.preferences, line.to_string(), 4),
                "facts" => push_unique_back(&mut sections.facts, line.to_string(), 4),
                "open_threads" => push_unique_back(&mut sections.open_threads, line.to_string(), 4),
                "recent_tools" => push_unique_back(&mut sections.recent_tools, line.to_string(), 4),
                _ => {}
            },
            _ => {}
        }
    }
    sections
}

fn render_sections(sections: &SummarySections, max_chars: usize) -> String {
    let mut working = sections.clone();
    loop {
        let rendered = render_all(&working);
        if rendered.chars().count() <= max_chars {
            return rendered;
        }
        if pop_last(&mut working.recent_tools)
            || pop_last(&mut working.open_threads)
            || pop_last(&mut working.facts)
        {
            continue;
        }
        return truncate_chars(&rendered, max_chars);
    }
}

fn render_all(sections: &SummarySections) -> String {
    let mut lines = vec!["[Session summary]".to_string()];
    append_section(&mut lines, "Preferences:", &sections.preferences);
    append_section(&mut lines, "Facts:", &sections.facts);
    append_section(&mut lines, "Open threads:", &sections.open_threads);
    append_section(&mut lines, "Recent tools:", &sections.recent_tools);
    lines.join("\n")
}

fn append_section(lines: &mut Vec<String>, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    lines.push(title.to_string());
    lines.extend(items.iter().cloned());
}

fn push_unique_front(entries: &mut Vec<String>, value: String, limit: usize) {
    let mut seen = BTreeSet::new();
    let _ = seen.insert(value.clone());
    let mut next = vec![value];
    for entry in entries.iter() {
        if seen.insert(entry.clone()) {
            next.push(entry.clone());
        }
    }
    next.dedup();
    next.truncate(limit);
    *entries = next;
}

fn push_unique_back(entries: &mut Vec<String>, value: String, limit: usize) {
    if entries.iter().any(|entry| entry == &value) {
        return;
    }
    entries.push(value);
    entries.truncate(limit);
}

fn pop_last(entries: &mut Vec<String>) -> bool {
    entries.pop().is_some()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect::<String>()
}
