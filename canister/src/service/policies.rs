//! where: standalone/canister/src/service/policies.rs
//! what: lightweight promotion, session-summary, and retry policies for the ICP agent flow
//! why: keep agent.rs focused on message/tool orchestration while isolating low-compute policy logic

#[path = "policies/promote.rs"]
mod promote;
#[path = "policies/summary.rs"]
mod summary;

use crate::context::retry_provider_once;
use crate::types::ContextConfig;
use iclaw_standalone_core::memory::Memory;
use iclaw_standalone_core::providers::ConversationMessage;
use std::sync::Arc;

pub(crate) use promote::{promoted_summary_line, PromotionCandidate, PromotionCategory};
pub(crate) use summary::is_summary_key;
pub(crate) use summary::record_conversation_turn;
pub(crate) use summary::replace_conversation_summary;
pub(crate) use summary::summary_key;

pub(crate) async fn load_conversation_summary(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<String> {
    summary::load_conversation_summary(memory, session_id, config).await
}

pub(crate) async fn refresh_conversation_summary(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    history: &[ConversationMessage],
    promoted: Option<&PromotionCandidate>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<()> {
    summary::refresh_conversation_summary(memory, session_id, history, promoted, config).await
}

pub(crate) async fn maybe_auto_promote(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    user_turn: &str,
    assistant_turn: &str,
    config: Option<&ContextConfig>,
) -> anyhow::Result<Option<PromotionCandidate>> {
    promote::maybe_auto_promote(memory, session_id, user_turn, assistant_turn, config).await
}

pub(crate) fn is_transient_provider_error(error: &anyhow::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("timeout")
        || message.contains("timed out")
        || message.contains("temporary transport failure")
        || message.contains("connection reset")
        || message.contains("api error (500)")
        || message.contains("api error (502)")
        || message.contains("api error (503)")
        || message.contains("api error (504)")
}

pub(crate) fn should_retry_once(config: Option<&ContextConfig>) -> bool {
    retry_provider_once(config)
}

#[cfg(test)]
#[path = "policies/tests.rs"]
mod tests;
