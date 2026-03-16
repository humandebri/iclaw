//! where: iclaw/canister/src/service/runs.rs
//! what: Memory-backed agent/session/run helpers for the phase-1 control plane
//! why: Keep run persistence and query logic isolated from the chat orchestration path

use crate::types::{PendingToolCall, Run, RunEvent, Session};
use chrono::Utc;
use iclaw_core::memory::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const SESSIONS_PREFIX: &str = "sessions/";
const RUNS_PREFIX: &str = "runs/";
const RUN_EVENTS_MARKER: &str = "/events/";
static RECORD_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredSession {
    id: String,
    agent_id: String,
    title: String,
    created_at: String,
    updated_at: String,
    last_run_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredRun {
    id: String,
    agent_id: String,
    session_id: String,
    status: String,
    prompt: String,
    response: Option<String>,
    model: Option<String>,
    requested_temperature: Option<f64>,
    provider_ready: bool,
    memory_ready: bool,
    created_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    error: Option<String>,
    trigger_kind: String,
    trigger_id: Option<String>,
    pending_tool_calls: Vec<StoredPendingToolCall>,
    pending_assistant_text: Option<String>,
    pending_reasoning_content: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredPendingToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredRunEvent {
    id: String,
    run_id: String,
    kind: String,
    message: String,
    timestamp: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ResumeState {
    pub run: Run,
    pub requested_temperature: Option<f64>,
    pub pending_reasoning_content: Option<String>,
}

pub(crate) async fn ensure_session(
    memory: &Arc<dyn Memory>,
    agent_id: &str,
    session_id: Option<&str>,
    prompt: &str,
) -> anyhow::Result<Session> {
    if let Some(session_id) = session_id.filter(|value| !value.trim().is_empty()) {
        if let Some(session) = get_session(Some(memory), session_id).await? {
            if session.agent_id != agent_id {
                return Err(anyhow::anyhow!(
                    "session agent_id does not match the requested agent_id"
                ));
            }
            return Ok(session);
        }
    }

    let now = now_text();
    let session = Session {
        id: session_id
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| next_id("session")),
        agent_id: agent_id.to_string(),
        title: summarize_prompt(prompt),
        created_at: now.clone(),
        updated_at: now,
        last_run_id: None,
    };
    store_session(memory, &session).await?;
    Ok(session)
}

pub(crate) async fn list_sessions(
    memory: Option<&Arc<dyn Memory>>,
    agent_id: Option<&str>,
) -> anyhow::Result<Vec<Session>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut sessions = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(SESSIONS_PREFIX) {
            continue;
        }
        let payload: StoredSession = serde_json::from_str(&entry.content)?;
        let session = payload.into_session();
        if agent_id.is_none_or(|value| session.agent_id == value) {
            sessions.push(session);
        }
    }
    sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(sessions)
}

pub(crate) async fn get_session(
    memory: Option<&Arc<dyn Memory>>,
    session_id: &str,
) -> anyhow::Result<Option<Session>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&session_key(session_id)).await? else {
        return Ok(None);
    };
    let payload: StoredSession = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_session()))
}

pub(crate) async fn create_run_record(
    memory: &Arc<dyn Memory>,
    session: &Session,
    prompt: &str,
    model: Option<&str>,
    temperature: Option<f64>,
    provider_ready: bool,
    memory_ready: bool,
    trigger_kind: &str,
    trigger_id: Option<&str>,
) -> anyhow::Result<Run> {
    let now = now_text();
    let run = Run {
        id: next_id("run"),
        agent_id: session.agent_id.clone(),
        session_id: session.id.clone(),
        status: "queued".to_string(),
        prompt: prompt.to_string(),
        response: None,
        model: model.map(str::to_string),
        provider_ready,
        memory_ready,
        created_at: now,
        started_at: None,
        finished_at: None,
        error: None,
        trigger_kind: trigger_kind.to_string(),
        trigger_id: trigger_id.map(str::to_string),
        pending_tool_calls: Vec::new(),
        pending_assistant_text: None,
    };
    store_run_state(memory, &run, temperature, None).await?;
    Ok(run)
}

pub(crate) async fn mark_run_started(memory: &Arc<dyn Memory>, run: &Run) -> anyhow::Result<Run> {
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    let next = Run {
        status: "running".to_string(),
        started_at: Some(now_text()),
        pending_tool_calls: Vec::new(),
        pending_assistant_text: None,
        ..run.clone()
    };
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        None,
    )
    .await?;
    Ok(next)
}

pub(crate) async fn mark_run_resumed(memory: &Arc<dyn Memory>, run: &Run) -> anyhow::Result<Run> {
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    let next = Run {
        status: "running".to_string(),
        finished_at: None,
        error: None,
        pending_tool_calls: Vec::new(),
        pending_assistant_text: None,
        ..run.clone()
    };
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        None,
    )
    .await?;
    Ok(next)
}

pub(crate) async fn mark_run_completed(
    memory: &Arc<dyn Memory>,
    run: &Run,
    response: &str,
    model: Option<&str>,
    provider_ready: bool,
    memory_ready: bool,
) -> anyhow::Result<Run> {
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    let next = Run {
        status: "completed".to_string(),
        response: Some(response.to_string()),
        model: model.map(str::to_string).or_else(|| run.model.clone()),
        provider_ready,
        memory_ready,
        finished_at: Some(now_text()),
        error: None,
        pending_tool_calls: Vec::new(),
        pending_assistant_text: None,
        ..run.clone()
    };
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        None,
    )
    .await?;
    Ok(next)
}

pub(crate) async fn mark_run_failed(
    memory: &Arc<dyn Memory>,
    run: &Run,
    error: &str,
    provider_ready: bool,
    memory_ready: bool,
) -> anyhow::Result<Run> {
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    let next = Run {
        status: "failed".to_string(),
        provider_ready,
        memory_ready,
        finished_at: Some(now_text()),
        error: Some(error.to_string()),
        pending_tool_calls: Vec::new(),
        pending_assistant_text: None,
        ..run.clone()
    };
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        None,
    )
    .await?;
    Ok(next)
}

pub(crate) async fn mark_run_blocked(
    memory: &Arc<dyn Memory>,
    run: &Run,
    error: &str,
    provider_ready: bool,
    memory_ready: bool,
    pending_tool_calls: &[PendingToolCall],
    pending_assistant_text: Option<&str>,
    pending_reasoning_content: Option<&str>,
) -> anyhow::Result<Run> {
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    let next = Run {
        status: "blocked".to_string(),
        provider_ready,
        memory_ready,
        finished_at: Some(now_text()),
        error: Some(error.to_string()),
        pending_tool_calls: pending_tool_calls.to_vec(),
        pending_assistant_text: pending_assistant_text.map(str::to_string),
        ..run.clone()
    };
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        pending_reasoning_content.map(str::to_string),
    )
    .await?;
    Ok(next)
}

pub(crate) async fn append_run_event(
    memory: &Arc<dyn Memory>,
    run: &Run,
    kind: &str,
    message: &str,
) -> anyhow::Result<RunEvent> {
    let event = RunEvent {
        id: next_id("event"),
        run_id: run.id.clone(),
        kind: kind.to_string(),
        message: message.to_string(),
        timestamp: now_text(),
    };
    let payload = serde_json::to_string(&StoredRunEvent::from_event(&event))?;
    memory
        .store(
            &run_event_key(&run.id, &event.id),
            &payload,
            MemoryCategory::Core,
            None,
        )
        .await?;
    Ok(event)
}

pub(crate) async fn touch_session_after_run(
    memory: &Arc<dyn Memory>,
    session: &Session,
    run: &Run,
) -> anyhow::Result<Session> {
    let next = Session {
        updated_at: now_text(),
        last_run_id: Some(run.id.clone()),
        ..session.clone()
    };
    store_session(memory, &next).await?;
    Ok(next)
}

pub(crate) async fn get_run(
    memory: Option<&Arc<dyn Memory>>,
    run_id: &str,
) -> anyhow::Result<Option<Run>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&run_key(run_id)).await? else {
        return Ok(None);
    };
    let payload: StoredRun = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_run()))
}

pub(crate) async fn get_resume_state(
    memory: Option<&Arc<dyn Memory>>,
    run_id: &str,
) -> anyhow::Result<Option<ResumeState>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&run_key(run_id)).await? else {
        return Ok(None);
    };
    let payload: StoredRun = serde_json::from_str(&entry.content)?;
    Ok(Some(ResumeState {
        run: payload.clone().into_run(),
        requested_temperature: payload.requested_temperature,
        pending_reasoning_content: payload.pending_reasoning_content,
    }))
}

pub(crate) async fn list_runs(
    memory: Option<&Arc<dyn Memory>>,
    session_id: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<Run>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut runs = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(RUNS_PREFIX) || entry.key.contains(RUN_EVENTS_MARKER) {
            continue;
        }
        let payload: StoredRun = serde_json::from_str(&entry.content)?;
        let run = payload.into_run();
        if session_id.is_none_or(|value| run.session_id == value) {
            runs.push(run);
        }
    }
    runs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    if limit < runs.len() {
        runs.truncate(limit);
    }
    Ok(runs)
}

pub(crate) async fn list_run_events(
    memory: Option<&Arc<dyn Memory>>,
    run_id: &str,
) -> anyhow::Result<Vec<RunEvent>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let prefix = format!(
        "{}/{}{}",
        RUNS_PREFIX.trim_end_matches('/'),
        run_id,
        RUN_EVENTS_MARKER
    );
    let mut events = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(&prefix) {
            continue;
        }
        let payload: StoredRunEvent = serde_json::from_str(&entry.content)?;
        events.push(payload.into_event());
    }
    events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
    Ok(events)
}

pub(crate) async fn cancel_run(memory: &Arc<dyn Memory>, run_id: &str) -> anyhow::Result<bool> {
    // Phase 1.1 only records cancellation in durable state; it does not interrupt an in-flight update call.
    let Some(run) = get_run(Some(memory), run_id).await? else {
        return Ok(false);
    };
    if run.status != "queued" && run.status != "running" {
        return Ok(false);
    }
    let next = Run {
        status: "cancelled".to_string(),
        finished_at: Some(now_text()),
        error: Some("run cancelled".to_string()),
        ..run.clone()
    };
    let resume_state = get_resume_state(Some(memory), &run.id).await?;
    store_run_state(
        memory,
        &next,
        resume_state
            .as_ref()
            .and_then(|state| state.requested_temperature),
        None,
    )
    .await?;
    append_run_event(memory, &next, "cancelled", "run cancelled").await?;
    if let Some(session) = get_session(Some(memory), &next.session_id).await? {
        let _ = touch_session_after_run(memory, &session, &next).await?;
    }
    Ok(true)
}

async fn store_session(memory: &Arc<dyn Memory>, session: &Session) -> anyhow::Result<()> {
    let payload = serde_json::to_string(&StoredSession::from_session(session))?;
    memory
        .store(
            &session_key(&session.id),
            &payload,
            MemoryCategory::Core,
            None,
        )
        .await
}

async fn store_run_state(
    memory: &Arc<dyn Memory>,
    run: &Run,
    requested_temperature: Option<f64>,
    pending_reasoning_content: Option<String>,
) -> anyhow::Result<()> {
    let payload = serde_json::to_string(&StoredRun::from_run(
        run,
        requested_temperature,
        pending_reasoning_content,
    ))?;
    memory
        .store(&run_key(&run.id), &payload, MemoryCategory::Core, None)
        .await
}

fn session_key(session_id: &str) -> String {
    format!("{SESSIONS_PREFIX}{session_id}")
}

fn run_key(run_id: &str) -> String {
    format!("{RUNS_PREFIX}{run_id}")
}

fn run_event_key(run_id: &str, event_id: &str) -> String {
    format!("{RUNS_PREFIX}{run_id}{RUN_EVENTS_MARKER}{event_id}")
}

fn next_id(prefix: &str) -> String {
    let sequence = RECORD_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{sequence}", Utc::now().timestamp_millis())
}

fn now_text() -> String {
    Utc::now().to_rfc3339()
}

fn summarize_prompt(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return "Untitled Session".to_string();
    }
    trimmed.chars().take(48).collect()
}

impl StoredSession {
    fn from_session(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            agent_id: session.agent_id.clone(),
            title: session.title.clone(),
            created_at: session.created_at.clone(),
            updated_at: session.updated_at.clone(),
            last_run_id: session.last_run_id.clone(),
        }
    }

    fn into_session(self) -> Session {
        Session {
            id: self.id,
            agent_id: self.agent_id,
            title: self.title,
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_run_id: self.last_run_id,
        }
    }
}

impl StoredRun {
    fn from_run(
        run: &Run,
        requested_temperature: Option<f64>,
        pending_reasoning_content: Option<String>,
    ) -> Self {
        Self {
            id: run.id.clone(),
            agent_id: run.agent_id.clone(),
            session_id: run.session_id.clone(),
            status: run.status.clone(),
            prompt: run.prompt.clone(),
            response: run.response.clone(),
            model: run.model.clone(),
            requested_temperature,
            provider_ready: run.provider_ready,
            memory_ready: run.memory_ready,
            created_at: run.created_at.clone(),
            started_at: run.started_at.clone(),
            finished_at: run.finished_at.clone(),
            error: run.error.clone(),
            trigger_kind: run.trigger_kind.clone(),
            trigger_id: run.trigger_id.clone(),
            pending_tool_calls: run
                .pending_tool_calls
                .iter()
                .cloned()
                .map(StoredPendingToolCall::from_pending_tool_call)
                .collect(),
            pending_assistant_text: run.pending_assistant_text.clone(),
            pending_reasoning_content,
        }
    }

    fn into_run(self) -> Run {
        Run {
            id: self.id,
            agent_id: self.agent_id,
            session_id: self.session_id,
            status: self.status,
            prompt: self.prompt,
            response: self.response,
            model: self.model,
            provider_ready: self.provider_ready,
            memory_ready: self.memory_ready,
            created_at: self.created_at,
            started_at: self.started_at,
            finished_at: self.finished_at,
            error: self.error,
            trigger_kind: self.trigger_kind,
            trigger_id: self.trigger_id,
            pending_tool_calls: self
                .pending_tool_calls
                .into_iter()
                .map(StoredPendingToolCall::into_pending_tool_call)
                .collect(),
            pending_assistant_text: self.pending_assistant_text,
        }
    }
}

impl StoredPendingToolCall {
    fn from_pending_tool_call(call: PendingToolCall) -> Self {
        Self {
            id: call.id,
            name: call.name,
            arguments: call.arguments,
        }
    }

    fn into_pending_tool_call(self) -> PendingToolCall {
        PendingToolCall {
            id: self.id,
            name: self.name,
            arguments: self.arguments,
        }
    }
}

impl StoredRunEvent {
    fn from_event(event: &RunEvent) -> Self {
        Self {
            id: event.id.clone(),
            run_id: event.run_id.clone(),
            kind: event.kind.clone(),
            message: event.message.clone(),
            timestamp: event.timestamp.clone(),
        }
    }

    fn into_event(self) -> RunEvent {
        RunEvent {
            id: self.id,
            run_id: self.run_id,
            kind: self.kind,
            message: self.message,
            timestamp: self.timestamp,
        }
    }
}
