//! where: iclaw/canister/src/service/agent.rs
//! what: multi-turn history, autosave, and explicit tool-loop helpers for the ICP service
//! why: keep service.rs focused on canister entrypoint orchestration rather than agent state flow

use super::compression::{compact_messages, CompressionState};
use super::policies::{is_transient_provider_error, record_conversation_turn, should_retry_once};
use crate::context::{
    enable_autosave, enable_tool_loop, history_limit, max_tool_iterations, PromptContext,
};
use crate::provider::{IcCanisterProvider, ProviderChatResult};
use crate::tools::IclawTool;
use crate::types::{ContextConfig, PendingToolCall, ToolPolicy};
use iclaw_core::memory::{Memory, MemoryCategory, MemoryEntry};
use iclaw_core::providers::{ChatMessage, ConversationMessage, ToolCall, ToolResultMessage};
use iclaw_core::tools::ToolSpec;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static TURN_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct HistoryRecord {
    key: String,
    message: ConversationMessage,
}

#[derive(Clone, Debug)]
struct ToolExecutionReport {
    tool_result: ToolResultMessage,
    had_failure: bool,
    event_kind: &'static str,
    event_message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ToolFailurePayload {
    error_code: &'static str,
    message: String,
    retryable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolLoopEvent {
    pub kind: String,
    pub message: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ToolAuthorization {
    pub agent_id: String,
    pub enabled_tool_names: Vec<String>,
    pub requires_tool_approval: bool,
    pub policies: Vec<ToolPolicy>,
}

#[derive(Clone, Debug)]
pub(crate) enum ToolLoopOutcome {
    Completed {
        response: ProviderChatResult,
        events: Vec<ToolLoopEvent>,
    },
    Blocked {
        message: String,
        events: Vec<ToolLoopEvent>,
        pending_tool_calls: Vec<PendingToolCall>,
        pending_assistant_text: Option<String>,
        pending_reasoning_content: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ApprovalCheck {
    Enforce,
    Skip,
}

pub(crate) fn tool_specs(tools: &[Box<dyn IclawTool>]) -> Vec<ToolSpec> {
    tools.iter().map(|tool| tool.spec()).collect()
}

pub(crate) fn build_messages(
    prompt_context: &PromptContext,
    session_summary: &str,
    history: &[ConversationMessage],
    prompt: &str,
) -> Vec<ConversationMessage> {
    let mut messages = Vec::new();
    messages.push(ConversationMessage::Chat(ChatMessage::system(
        prompt_context.system_prompt.clone(),
    )));
    if !prompt_context.memory_context.trim().is_empty() {
        messages.push(ConversationMessage::Chat(ChatMessage::user(
            prompt_context.memory_context.clone(),
        )));
    }
    if !session_summary.trim().is_empty() {
        messages.push(ConversationMessage::Chat(ChatMessage::user(
            session_summary.to_string(),
        )));
    }
    messages.extend(history.iter().cloned());
    messages.push(ConversationMessage::Chat(ChatMessage::user(
        prompt.to_string(),
    )));
    messages
}

pub(crate) fn build_messages_without_prompt(
    prompt_context: &PromptContext,
    session_summary: &str,
    history: &[ConversationMessage],
) -> Vec<ConversationMessage> {
    let mut messages = Vec::new();
    messages.push(ConversationMessage::Chat(ChatMessage::system(
        prompt_context.system_prompt.clone(),
    )));
    if !prompt_context.memory_context.trim().is_empty() {
        messages.push(ConversationMessage::Chat(ChatMessage::user(
            prompt_context.memory_context.clone(),
        )));
    }
    if !session_summary.trim().is_empty() {
        messages.push(ConversationMessage::Chat(ChatMessage::user(
            session_summary.to_string(),
        )));
    }
    messages.extend(history.iter().cloned());
    messages
}

pub(crate) async fn load_history(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    config: Option<&ContextConfig>,
) -> anyhow::Result<Vec<ConversationMessage>> {
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(Vec::new());
    };
    let limit = history_limit(config);
    let mut history = list_history_records(memory, session_id)
        .await?
        .into_iter()
        .map(|record| record.message)
        .collect::<Vec<_>>();
    if history.len() > limit {
        history.drain(0..history.len() - limit);
    }
    Ok(history)
}

pub(crate) async fn autosave_turn(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    role: &str,
    content: &str,
    config: Option<&ContextConfig>,
) -> anyhow::Result<()> {
    if !enable_autosave(config) || content.trim().is_empty() {
        return Ok(());
    }
    let (Some(memory), Some(session_id)) = (memory, session_id) else {
        return Ok(());
    };
    let key = format!(
        "conversation/{}/{}/{}",
        sanitize_session_key(session_id),
        role,
        unique_turn_suffix()
    );
    memory
        .store(
            &key,
            content,
            MemoryCategory::Conversation,
            Some(session_id),
        )
        .await?;
    record_conversation_turn(Some(memory), Some(session_id)).await?;
    prune_history(memory, session_id, history_limit(config)).await
}

pub(crate) async fn run_tool_loop(
    provider: &Arc<dyn IcCanisterProvider>,
    tools: &[Box<dyn IclawTool>],
    history: &mut Vec<ConversationMessage>,
    model: &str,
    temperature: f64,
    authorization: &ToolAuthorization,
    config: Option<&ContextConfig>,
    compression_state: &mut CompressionState,
) -> anyhow::Result<ToolLoopOutcome> {
    let specs = tool_specs(tools);
    let allow_tools = enable_tool_loop(config);
    let iterations = max_tool_iterations(config);
    if !allow_tools {
        return chat_with_retry(
            provider,
            history,
            None,
            model,
            temperature,
            config,
            compression_state,
        )
        .await
        .map(|response| ToolLoopOutcome::Completed {
            response,
            events: Vec::new(),
        });
    }
    if !provider.capabilities().native_tool_calling {
        anyhow::bail!("Configured provider does not support native tool calling");
    }

    let mut previous_failed_signature = None::<String>;
    let mut events = Vec::new();
    for attempt in 0..=iterations {
        let response = chat_with_retry(
            provider,
            history,
            Some(&specs),
            model,
            temperature,
            config,
            compression_state,
        )
        .await?;
        if response.response.tool_calls.is_empty() {
            return Ok(ToolLoopOutcome::Completed { response, events });
        }
        if attempt == iterations {
            anyhow::bail!("tool loop exceeded configured iteration limit ({iterations})");
        }
        let current_signature = tool_call_signature(&response.response.tool_calls);
        if previous_failed_signature.as_deref() == Some(current_signature.as_str()) {
            anyhow::bail!("tool loop repeated the same failing call");
        }
        history.push(ConversationMessage::AssistantToolCalls {
            text: response.response.text.clone(),
            tool_calls: response.response.tool_calls.clone(),
            reasoning_content: response.response.reasoning_content.clone(),
        });
        for call in &response.response.tool_calls {
            events.push(ToolLoopEvent {
                kind: "tool_requested".to_string(),
                message: format!("tool '{}' requested", call.name),
            });
        }
        if let Some(message) = first_blocking_policy(
            authorization,
            &response.response.tool_calls,
            ApprovalCheck::Enforce,
        ) {
            for call in &response.response.tool_calls {
                if let Some(blocked_message) =
                    blocking_message_for_call(authorization, call, ApprovalCheck::Enforce)
                {
                    events.push(ToolLoopEvent {
                        kind: "tool_blocked".to_string(),
                        message: blocked_message,
                    });
                }
            }
            return Ok(ToolLoopOutcome::Blocked {
                message,
                events,
                pending_tool_calls: response
                    .response
                    .tool_calls
                    .iter()
                    .map(|call| PendingToolCall {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                    })
                    .collect(),
                pending_assistant_text: response.response.text.clone(),
                pending_reasoning_content: response.response.reasoning_content.clone(),
            });
        }
        let reports = execute_tool_calls(tools, &response.response.tool_calls).await;
        let had_failure = reports.iter().any(|report| report.had_failure);
        for report in &reports {
            events.push(ToolLoopEvent {
                kind: report.event_kind.to_string(),
                message: report.event_message.clone(),
            });
        }
        history.push(ConversationMessage::ToolResults(
            reports
                .into_iter()
                .map(|report| report.tool_result)
                .collect(),
        ));
        previous_failed_signature = had_failure.then_some(current_signature);
    }

    anyhow::bail!("tool loop exited unexpectedly")
}

pub(crate) async fn resume_tool_loop(
    provider: &Arc<dyn IcCanisterProvider>,
    tools: &[Box<dyn IclawTool>],
    history: &mut Vec<ConversationMessage>,
    model: &str,
    temperature: f64,
    authorization: &ToolAuthorization,
    config: Option<&ContextConfig>,
    compression_state: &mut CompressionState,
    pending_tool_calls: &[PendingToolCall],
    pending_assistant_text: Option<&str>,
    pending_reasoning_content: Option<&str>,
) -> anyhow::Result<ToolLoopOutcome> {
    let restored_calls = pending_tool_calls
        .iter()
        .map(|call| ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        })
        .collect::<Vec<_>>();
    if let Some(message) =
        first_blocking_policy(authorization, &restored_calls, ApprovalCheck::Skip)
    {
        let events = restored_calls
            .iter()
            .filter_map(|call| {
                blocking_message_for_call(authorization, call, ApprovalCheck::Skip).map(
                    |blocked_message| ToolLoopEvent {
                        kind: "tool_blocked".to_string(),
                        message: blocked_message,
                    },
                )
            })
            .collect::<Vec<_>>();
        return Ok(ToolLoopOutcome::Blocked {
            message,
            events,
            pending_tool_calls: pending_tool_calls.to_vec(),
            pending_assistant_text: pending_assistant_text.map(str::to_string),
            pending_reasoning_content: pending_reasoning_content.map(str::to_string),
        });
    }
    history.push(ConversationMessage::AssistantToolCalls {
        text: pending_assistant_text.map(str::to_string),
        tool_calls: restored_calls.clone(),
        reasoning_content: pending_reasoning_content.map(str::to_string),
    });

    let reports = execute_tool_calls(tools, &restored_calls).await;
    let mut events = reports
        .iter()
        .map(|report| ToolLoopEvent {
            kind: report.event_kind.to_string(),
            message: report.event_message.clone(),
        })
        .collect::<Vec<_>>();
    history.push(ConversationMessage::ToolResults(
        reports
            .into_iter()
            .map(|report| report.tool_result)
            .collect(),
    ));

    match run_tool_loop(
        provider,
        tools,
        history,
        model,
        temperature,
        authorization,
        config,
        compression_state,
    )
    .await?
    {
        ToolLoopOutcome::Completed {
            response,
            events: mut next_events,
        } => {
            events.append(&mut next_events);
            Ok(ToolLoopOutcome::Completed { response, events })
        }
        ToolLoopOutcome::Blocked {
            message,
            events: mut next_events,
            pending_tool_calls,
            pending_assistant_text,
            pending_reasoning_content,
        } => {
            events.append(&mut next_events);
            Ok(ToolLoopOutcome::Blocked {
                message,
                events,
                pending_tool_calls,
                pending_assistant_text,
                pending_reasoning_content,
            })
        }
    }
}

async fn chat_with_retry(
    provider: &Arc<dyn IcCanisterProvider>,
    messages: &[ConversationMessage],
    tools: Option<&[ToolSpec]>,
    model: &str,
    temperature: f64,
    config: Option<&ContextConfig>,
    compression_state: &mut CompressionState,
) -> anyhow::Result<ProviderChatResult> {
    let compacted = compact_messages(
        provider,
        messages,
        tools,
        model,
        temperature,
        config,
        compression_state,
    )
    .await;
    match provider.chat(&compacted, tools, model, temperature).await {
        Ok(response) => Ok(response),
        Err(error) if should_retry_once(config) && is_transient_provider_error(&error) => {
            let compacted = compact_messages(
                provider,
                messages,
                tools,
                model,
                temperature,
                config,
                compression_state,
            )
            .await;
            provider.chat(&compacted, tools, model, temperature).await
        }
        Err(error) => Err(error),
    }
}

async fn prune_history(
    memory: &Arc<dyn Memory>,
    session_id: &str,
    limit: usize,
) -> anyhow::Result<()> {
    let records = list_history_records(memory, session_id).await?;
    let excess = records.len().saturating_sub(limit);
    for record in records.into_iter().take(excess) {
        memory.forget(&record.key).await?;
    }
    Ok(())
}

async fn list_history_records(
    memory: &Arc<dyn Memory>,
    session_id: &str,
) -> anyhow::Result<Vec<HistoryRecord>> {
    let mut entries = memory
        .list(Some(&MemoryCategory::Conversation), Some(session_id))
        .await?;
    entries.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.key.cmp(&right.key))
    });
    Ok(entries
        .into_iter()
        .filter_map(parse_history_entry)
        .collect())
}

fn parse_history_entry(entry: MemoryEntry) -> Option<HistoryRecord> {
    if !entry.key.starts_with("conversation/") || entry.content.trim().is_empty() {
        return None;
    }
    if entry.key.contains("/user/") {
        return Some(HistoryRecord {
            key: entry.key,
            message: ConversationMessage::Chat(ChatMessage::user(entry.content)),
        });
    }
    if entry.key.contains("/assistant/") {
        return Some(HistoryRecord {
            key: entry.key,
            message: ConversationMessage::Chat(ChatMessage::assistant(entry.content)),
        });
    }
    None
}

async fn execute_tool_calls(
    tools: &[Box<dyn IclawTool>],
    calls: &[iclaw_core::providers::ToolCall],
) -> Vec<ToolExecutionReport> {
    let mut results = Vec::new();
    for call in calls {
        let report = if let Some(tool) = tools.iter().find(|tool| tool.name() == call.name) {
            match parse_arguments(&call.arguments) {
                Ok(args) => match tool.execute(args).await {
                    Ok(result) if result.success => successful_tool_result(call, result.output),
                    Ok(result) => failed_tool_result(
                        call,
                        classify_tool_error(result.error.unwrap_or_else(|| {
                            "tool execution returned an unsuccessful result".to_string()
                        })),
                    ),
                    Err(error) => failed_tool_result(call, classify_tool_error(error.to_string())),
                },
                Err(error) => failed_tool_result(
                    call,
                    ToolFailurePayload {
                        error_code: "invalid_arguments",
                        message: error.to_string(),
                        retryable: false,
                    },
                ),
            }
        } else {
            failed_tool_result(
                call,
                ToolFailurePayload {
                    error_code: "unknown_tool",
                    message: format!("unknown tool '{}'", call.name),
                    retryable: false,
                },
            )
        };
        results.push(report);
    }
    results
}

fn parse_arguments(raw: &str) -> anyhow::Result<serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(value) if value.is_object() => Ok(value),
        Ok(_) => anyhow::bail!("tool arguments must decode to a JSON object"),
        Err(error) => anyhow::bail!("invalid tool arguments JSON: {error}"),
    }
}

fn successful_tool_result(
    call: &iclaw_core::providers::ToolCall,
    output: String,
) -> ToolExecutionReport {
    ToolExecutionReport {
        had_failure: false,
        event_kind: "tool_succeeded",
        event_message: format!("tool '{}' succeeded", call.name),
        tool_result: ToolResultMessage {
            tool_call_id: call.id.clone(),
            content: serde_json::json!({
                "success": true,
                "output": output,
                "error_code": serde_json::Value::Null,
                "message": serde_json::Value::Null,
                "retryable": false,
            })
            .to_string(),
        },
    }
}

fn failed_tool_result(
    call: &iclaw_core::providers::ToolCall,
    payload: ToolFailurePayload,
) -> ToolExecutionReport {
    ToolExecutionReport {
        had_failure: true,
        event_kind: "tool_failed",
        event_message: format!("tool '{}' failed: {}", call.name, payload.message),
        tool_result: ToolResultMessage {
            tool_call_id: call.id.clone(),
            content: serde_json::json!({
                "success": false,
                "output": "",
                "error_code": payload.error_code,
                "message": payload.message,
                "retryable": payload.retryable,
            })
            .to_string(),
        },
    }
}

fn classify_tool_error(message: String) -> ToolFailurePayload {
    let lowered = message.to_ascii_lowercase();
    if lowered.contains("allowlist") {
        return ToolFailurePayload {
            error_code: "http_allowlist_denied",
            message,
            retryable: false,
        };
    }
    ToolFailurePayload {
        error_code: "tool_execution_failed",
        message,
        retryable: false,
    }
}

fn tool_call_signature(calls: &[iclaw_core::providers::ToolCall]) -> String {
    calls
        .iter()
        .map(|call| format!("{}:{}", call.name, call.arguments))
        .collect::<Vec<_>>()
        .join("|")
}

fn first_blocking_policy(
    authorization: &ToolAuthorization,
    calls: &[ToolCall],
    approval_check: ApprovalCheck,
) -> Option<String> {
    calls
        .iter()
        .find_map(|call| blocking_message_for_call(authorization, call, approval_check))
}

fn blocking_message_for_call(
    authorization: &ToolAuthorization,
    call: &ToolCall,
    approval_check: ApprovalCheck,
) -> Option<String> {
    if !authorization
        .enabled_tool_names
        .iter()
        .any(|name| name == &call.name)
    {
        return Some(format!(
            "tool '{}' is disabled for agent '{}'",
            call.name, authorization.agent_id
        ));
    }
    let policy = authorization
        .policies
        .iter()
        .find(|policy| policy.tool_name == call.name);
    if let Some(policy) = policy {
        if !policy.enabled {
            return Some(format!("tool '{}' is disabled by policy", call.name));
        }
    }
    if approval_check == ApprovalCheck::Enforce
        && (authorization.requires_tool_approval
            || policy.is_some_and(|policy| policy.requires_approval))
    {
        return Some(format!("tool '{}' requires approval", call.name));
    }
    None
}

pub(crate) fn sanitize_session_key(session_id: &str) -> String {
    session_id.replace('/', "~")
}

fn unique_turn_suffix() -> String {
    let counter = TURN_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{counter}", now_nanos())
}

#[cfg(target_arch = "wasm32")]
fn now_nanos() -> u64 {
    ic_cdk::api::time()
}

#[cfg(not(target_arch = "wasm32"))]
fn now_nanos() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}
