//! where: standalone/canister/src/service/observe.rs
//! what: lightweight query-only observation helpers for the ICP agent runtime
//! why: expose summary and memory-backed operating state without adding heavy indexes or new storage

use super::policies;
use crate::context::{
    enable_auto_promote, enable_conversation_summary, enable_tool_loop, history_limit,
    max_tool_iterations,
};
use crate::types::{AgentObservation, ContextConfig, MemoryItem};
use iclaw_standalone_core::memory::{Memory, MemoryCategory};
use std::sync::Arc;

const AUTO_PROMOTED_PREFIXES: [&str; 3] = [
    "core/user_preferences/",
    "core/project_facts/",
    "core/recurring_goals/",
];

pub(crate) async fn conversation_summary_item(
    memory: Option<&Arc<dyn Memory>>,
    session_id: &str,
) -> anyhow::Result<Option<MemoryItem>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let key = policies::summary_key(session_id);
    memory.get(&key).await.map(|entry| entry.map(Into::into))
}

pub(crate) async fn observe(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<AgentObservation> {
    let Some(memory) = memory else {
        return Ok(empty_observation(session_id, config));
    };

    let core_entries = memory.list(Some(&MemoryCategory::Core), None).await?;
    let workspace_keys = core_entries
        .iter()
        .filter(|entry| entry.key.starts_with("workspace/"))
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    let core_keys = core_entries
        .iter()
        .filter(|entry| !entry.key.starts_with("workspace/"))
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    let auto_promoted_keys = core_keys
        .iter()
        .filter(|key| {
            AUTO_PROMOTED_PREFIXES
                .iter()
                .any(|prefix| key.starts_with(prefix))
        })
        .cloned()
        .collect::<Vec<_>>();

    let (conversation_summary_key, conversation_summary_present, conversation_turn_count) =
        match session_id {
            Some(session_id) => {
                let summary_key = policies::summary_key(session_id);
                let conversation_entries = memory
                    .list(Some(&MemoryCategory::Conversation), Some(session_id))
                    .await?;
                let turn_count = conversation_entries
                    .iter()
                    .filter(|entry| entry.key.starts_with("conversation/"))
                    .count() as u64;
                let summary_present = conversation_entries
                    .iter()
                    .any(|entry| entry.key == summary_key);
                (Some(summary_key), summary_present, turn_count)
            }
            None => (None, false, 0),
        };

    Ok(AgentObservation {
        workspace_keys,
        core_keys,
        conversation_summary_key,
        conversation_summary_present,
        conversation_turn_count,
        auto_promoted_keys,
        history_limit: history_limit(config) as u64,
        enable_auto_promote: enable_auto_promote(config),
        enable_conversation_summary: enable_conversation_summary(config),
        tool_loop_enabled: enable_tool_loop(config),
        max_tool_iterations: max_tool_iterations(config) as u64,
    })
}

fn empty_observation(session_id: Option<&str>, config: Option<&ContextConfig>) -> AgentObservation {
    AgentObservation {
        workspace_keys: Vec::new(),
        core_keys: Vec::new(),
        conversation_summary_key: session_id.map(policies::summary_key),
        conversation_summary_present: false,
        conversation_turn_count: 0,
        auto_promoted_keys: Vec::new(),
        history_limit: history_limit(config) as u64,
        enable_auto_promote: enable_auto_promote(config),
        enable_conversation_summary: enable_conversation_summary(config),
        tool_loop_enabled: enable_tool_loop(config),
        max_tool_iterations: max_tool_iterations(config) as u64,
    }
}
