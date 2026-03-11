//! where: standalone/canister/src/service/policies/tests.rs
//! what: focused tests for auto-promotion, summary parsing, and retry classification
//! why: keep policy behavior deterministic while the canister agent remains lightweight

use super::promote::{detect_durable_fact, detect_preference, detect_recurring_goal};
use super::summary::{build_summary, parse_summary_turn_count};
use super::*;
use iclaw_standalone_core::providers::ChatMessage;
use iclaw_standalone_core::providers::ToolResultMessage;

#[test]
fn detects_preference_conservatively_in_english_and_japanese() {
    assert_eq!(
        detect_preference("Prefer concise answers with concrete implementation details.")
            .map(|candidate| candidate.key),
        Some("core/user_preferences/response_style".to_string())
    );
    assert_eq!(
        detect_preference("Lead with the conclusion and then explain the implementation.")
            .map(|candidate| candidate.key),
        Some("core/user_preferences/answer_structure".to_string())
    );
    assert_eq!(
        detect_preference("Include test results in every implementation update.")
            .map(|candidate| candidate.key),
        Some("core/user_preferences/verification_level".to_string())
    );
    assert_eq!(detect_preference("日本語で答えて"), None);
    assert_eq!(detect_preference("hello there"), None);
}

#[test]
fn detects_fact_and_goal_only_from_explicit_phrases() {
    assert_eq!(
        detect_durable_fact("Remember that the runtime is ICP-only", "hello")
            .map(|candidate| candidate.key),
        Some("core/project_facts/remembered".to_string())
    );
    assert_eq!(
        detect_durable_fact("We are working on the ICP canister runtime only.", "hello")
            .map(|candidate| candidate.key),
        Some("core/project_facts/working_assumption".to_string())
    );
    assert_eq!(
        detect_recurring_goal("Every time we work on ICP, include the latest verification status.")
            .map(|candidate| candidate.key),
        Some("core/recurring_goals/reminder".to_string())
    );
    assert_eq!(
        detect_durable_fact("このプロジェクトでは canister first で進める", "hello"),
        None
    );
    assert_eq!(
        detect_durable_fact("hello", "iclaw_ic runs as an ICP-canister")
            .map(|candidate| candidate.key),
        None
    );
    assert_eq!(detect_durable_fact("that sounds good", "hello"), None);
}

#[test]
fn parses_and_filters_summary_keys() {
    let parsed = parse_summary_turn_count(
        "turn_count:8\nsummary_turn_count:8\n[Session summary]\nPreferences:\n- concise\nFacts:\n- runtime",
    );
    assert_eq!(parsed, Some(8));
    assert!(is_summary_key("conversation_summary/session-a"));
}

#[test]
fn parses_legacy_summary_format_without_refresh_marker() {
    let parsed =
        parse_summary_turn_count("turn_count:6\n[Session summary]\nFacts:\n- legacy summary");
    assert_eq!(parsed, Some(6));
}

#[test]
fn summary_generation_stays_structured_and_prioritizes_preferences() {
    let summary = build_summary(
        &[
            ConversationMessage::Chat(ChatMessage::user(
                "Prefer concise answers with concrete implementation details.".to_string(),
            )),
            ConversationMessage::Chat(ChatMessage::assistant(
                "iclaw runs as an icp-canister and uses non-replicated provider HTTP outcalls."
                    .to_string(),
            )),
            ConversationMessage::AssistantToolCalls {
                text: Some("checking memory".to_string()),
                tool_calls: vec![],
                reasoning_content: None,
            },
            ConversationMessage::ToolResults(vec![ToolResultMessage {
                tool_call_id: "call-1".to_string(),
                content:
                    "{\"success\":false,\"output\":\"\",\"error_code\":\"unknown_tool\",\"message\":\"bad\",\"retryable\":false}"
                        .to_string(),
            }]),
        ],
        Some(
            "[Session summary]\nPreferences:\n- keep prior preference\nFacts:\n- durable runtime",
        ),
        Some(&PromotionCandidate {
            key: "core/user_preferences/response_style".to_string(),
            content: "Prefer concise answers with concrete implementation details.".to_string(),
            category: PromotionCategory::Preference,
        }),
        320,
    );
    assert!(summary.contains("[Session summary]"));
    assert!(summary.contains("Preferences:"));
    assert!(summary.contains("Prefer concise answers"));
    assert!(summary.chars().count() <= 320);
}

#[test]
fn transient_provider_error_detection_is_narrow() {
    assert!(is_transient_provider_error(&anyhow::anyhow!(
        "temporary transport failure"
    )));
    assert!(is_transient_provider_error(&anyhow::anyhow!(
        "OpenAI-compatible API error (503): upstream unavailable"
    )));
    assert!(!is_transient_provider_error(&anyhow::anyhow!(
        "Host 'example.com' is not in the configured allowlist"
    )));
}
