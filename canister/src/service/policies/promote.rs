//! where: iclaw/canister/src/service/policies/promote.rs
//! what: conservative long-term memory promotion rules for the ICP chat flow
//! why: keep auto-promotion deterministic, low-compute, and easy to audit

use crate::context::enable_auto_promote;
use crate::types::ContextConfig;
use iclaw_core::memory::{Memory, MemoryCategory};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromotionCategory {
    Preference,
    Fact,
    Goal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PromotionCandidate {
    pub key: String,
    pub content: String,
    pub category: PromotionCategory,
}

pub(crate) async fn maybe_auto_promote(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    user_turn: &str,
    assistant_turn: &str,
    config: Option<&ContextConfig>,
) -> anyhow::Result<Option<PromotionCandidate>> {
    if !enable_auto_promote(config) {
        return Ok(None);
    }
    let (Some(memory), Some(_session_id)) = (memory, session_id) else {
        return Ok(None);
    };
    let Some(candidate) = detect_preference(user_turn)
        .or_else(|| detect_recurring_goal(user_turn))
        .or_else(|| detect_durable_fact(user_turn, assistant_turn))
    else {
        return Ok(None);
    };
    if memory
        .get(&candidate.key)
        .await?
        .is_some_and(|entry| entry.content.trim() == candidate.content.trim())
    {
        return Ok(Some(candidate));
    }
    memory
        .store(
            &candidate.key,
            &candidate.content,
            MemoryCategory::Core,
            None,
        )
        .await?;
    Ok(Some(candidate))
}

pub(crate) fn promoted_summary_line(candidate: &PromotionCandidate) -> String {
    match candidate.category {
        PromotionCategory::Preference => format!("- {}", candidate.content),
        PromotionCategory::Fact => format!("- {}", candidate.content),
        PromotionCategory::Goal => format!("- {}", candidate.content),
    }
}

pub(super) fn detect_preference(text: &str) -> Option<PromotionCandidate> {
    let trimmed = text.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.contains("prefer concise") || lowered.contains("keep answers concise") {
        return Some(PromotionCandidate {
            key: "core/user_preferences/response_style".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Preference,
        });
    }
    if lowered.contains("concrete implementation details") {
        return Some(PromotionCandidate {
            key: "core/user_preferences/response_style".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Preference,
        });
    }
    if lowered.contains("lead with the conclusion") {
        return Some(PromotionCandidate {
            key: "core/user_preferences/answer_structure".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Preference,
        });
    }
    if lowered.contains("include test results") || lowered.contains("report test results") {
        return Some(PromotionCandidate {
            key: "core/user_preferences/verification_level".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Preference,
        });
    }
    None
}

pub(super) fn detect_recurring_goal(text: &str) -> Option<PromotionCandidate> {
    let trimmed = text.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.contains("ongoing goal") || lowered.contains("recurring goal") {
        return Some(PromotionCandidate {
            key: "core/recurring_goals/current".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Goal,
        });
    }
    if lowered.contains("always remind me") || lowered.contains("every time we work on") {
        return Some(PromotionCandidate {
            key: "core/recurring_goals/reminder".to_string(),
            content: trimmed.to_string(),
            category: PromotionCategory::Goal,
        });
    }
    None
}

pub(super) fn detect_durable_fact(
    user_turn: &str,
    _assistant_turn: &str,
) -> Option<PromotionCandidate> {
    let trimmed_user = user_turn.trim();
    let lowered_user = trimmed_user.to_ascii_lowercase();
    if let Some(content) =
        extract_prefixed_content(trimmed_user, &["Remember that ", "remember that "])
    {
        return Some(PromotionCandidate {
            key: "core/project_facts/remembered".to_string(),
            content,
            category: PromotionCategory::Fact,
        });
    }
    if lowered_user.contains("we are working on ") {
        return Some(PromotionCandidate {
            key: "core/project_facts/working_assumption".to_string(),
            content: trimmed_user.to_string(),
            category: PromotionCategory::Fact,
        });
    }
    None
}

fn extract_prefixed_content(text: &str, prefixes: &[&str]) -> Option<String> {
    prefixes.iter().find_map(|prefix| {
        text.strip_prefix(prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    })
}

pub(super) fn first_important_sentence(text: &str) -> Option<&str> {
    text.split(['\n', '.', '!', '?', '。'])
        .map(str::trim)
        .find(|line| !line.is_empty() && line.len() > 8)
}
