//! where: iclaw/canister/src/service/schedules.rs
//! what: Durable schedule records and execution-state transitions for timer-driven runs
//! why: keep schedule persistence and lifecycle updates separate from service orchestration

use crate::types::{
    Run, Schedule, ScheduleCreateRequest, ScheduleDraft, ScheduleGetRequest, ScheduleUpdateRequest,
};
use chrono::{DateTime, Duration, Utc};
use iclaw_core::memory::{Memory, MemoryCategory};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const SCHEDULES_PREFIX: &str = "schedules/";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredSchedule {
    id: String,
    name: String,
    agent_id: String,
    prompt: String,
    interval_minutes: u64,
    session_mode: String,
    fixed_session_id: Option<String>,
    enabled: bool,
    created_at: String,
    updated_at: String,
    next_run_at: Option<String>,
    last_started_at: Option<String>,
    last_finished_at: Option<String>,
    last_run_id: Option<String>,
    last_error: Option<String>,
    consecutive_failure_count: u64,
    last_success_at: Option<String>,
    running: bool,
}

pub(crate) async fn list_schedules(
    memory: Option<&Arc<dyn Memory>>,
) -> anyhow::Result<Vec<Schedule>> {
    let Some(memory) = memory else {
        return Ok(Vec::new());
    };
    let mut schedules = Vec::new();
    for entry in memory.list(Some(&MemoryCategory::Core), None).await? {
        if !entry.key.starts_with(SCHEDULES_PREFIX) {
            continue;
        }
        let payload: StoredSchedule = serde_json::from_str(&entry.content)?;
        schedules.push(payload.into_schedule());
    }
    schedules.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(schedules)
}

pub(crate) async fn get_schedule(
    memory: Option<&Arc<dyn Memory>>,
    request: &ScheduleGetRequest,
) -> anyhow::Result<Option<Schedule>> {
    let Some(memory) = memory else {
        return Ok(None);
    };
    let Some(entry) = memory.get(&schedule_key(&request.schedule_id)).await? else {
        return Ok(None);
    };
    let payload: StoredSchedule = serde_json::from_str(&entry.content)?;
    Ok(Some(payload.into_schedule()))
}

pub(crate) async fn create_schedule(
    memory: &Arc<dyn Memory>,
    request: ScheduleCreateRequest,
) -> anyhow::Result<Schedule> {
    if get_schedule(
        Some(memory),
        &ScheduleGetRequest {
            schedule_id: request.draft.id.clone(),
        },
    )
    .await?
    .is_some()
    {
        anyhow::bail!("schedule already exists");
    }
    let schedule = Schedule::from(request.draft);
    store_schedule(memory, &schedule).await?;
    Ok(schedule)
}

pub(crate) async fn update_schedule(
    memory: &Arc<dyn Memory>,
    request: ScheduleUpdateRequest,
) -> anyhow::Result<Schedule> {
    let existing = get_schedule(
        Some(memory),
        &ScheduleGetRequest {
            schedule_id: request.schedule.id.clone(),
        },
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("schedule does not exist"))?;
    let interval_changed = existing.interval_minutes != request.schedule.interval_minutes;
    let next_run_at = if !request.schedule.enabled {
        None
    } else if !existing.enabled || existing.next_run_at.is_none() || interval_changed {
        Some(next_run_after(Utc::now(), request.schedule.interval_minutes))
    } else {
        existing.next_run_at.clone()
    };
    let schedule = Schedule {
        id: existing.id,
        name: request.schedule.name,
        agent_id: request.schedule.agent_id,
        prompt: request.schedule.prompt,
        interval_minutes: request.schedule.interval_minutes,
        session_mode: request.schedule.session_mode,
        fixed_session_id: request.schedule.fixed_session_id,
        enabled: request.schedule.enabled,
        created_at: existing.created_at,
        updated_at: now_text(),
        next_run_at,
        last_started_at: existing.last_started_at,
        last_finished_at: existing.last_finished_at,
        last_run_id: existing.last_run_id,
        last_error: existing.last_error,
        consecutive_failure_count: existing.consecutive_failure_count,
        last_success_at: existing.last_success_at,
        running: existing.running,
    };
    store_schedule(memory, &schedule).await?;
    Ok(schedule)
}

pub(crate) async fn delete_schedule(
    memory: &Arc<dyn Memory>,
    schedule_id: &str,
) -> anyhow::Result<bool> {
    memory.forget(&schedule_key(schedule_id)).await
}

pub(crate) async fn mark_schedule_running(
    memory: &Arc<dyn Memory>,
    schedule: &Schedule,
) -> anyhow::Result<Schedule> {
    let next = Schedule {
        updated_at: now_text(),
        last_started_at: Some(now_text()),
        last_error: None,
        running: true,
        ..schedule.clone()
    };
    store_schedule(memory, &next).await?;
    Ok(next)
}

pub(crate) async fn mark_schedule_skipped(
    memory: &Arc<dyn Memory>,
    schedule: &Schedule,
    reason: &str,
) -> anyhow::Result<Schedule> {
    let next = Schedule {
        updated_at: now_text(),
        last_error: Some(reason.to_string()),
        next_run_at: Some(next_run_after(Utc::now(), schedule.interval_minutes)),
        consecutive_failure_count: schedule.consecutive_failure_count.saturating_add(1),
        ..schedule.clone()
    };
    store_schedule(memory, &next).await?;
    Ok(next)
}

pub(crate) async fn mark_schedule_finished(
    memory: &Arc<dyn Memory>,
    schedule: &Schedule,
    run: Option<&Run>,
    error: Option<&str>,
    advance_next_run: bool,
) -> anyhow::Result<Schedule> {
    let now = Utc::now();
    let succeeded = run.is_some_and(schedule_run_succeeded) && error.is_none();
    let last_error = error
        .map(str::to_string)
        .or_else(|| run.and_then(|value| value.error.clone()));
    let next = Schedule {
        updated_at: now.to_rfc3339(),
        next_run_at: if schedule.enabled && advance_next_run {
            Some(next_run_after(now, schedule.interval_minutes))
        } else {
            schedule.next_run_at.clone()
        },
        last_finished_at: Some(now.to_rfc3339()),
        last_run_id: run.map(|value| value.id.clone()).or_else(|| schedule.last_run_id.clone()),
        last_error,
        consecutive_failure_count: if succeeded {
            0
        } else {
            schedule.consecutive_failure_count.saturating_add(1)
        },
        last_success_at: if succeeded {
            Some(now.to_rfc3339())
        } else {
            schedule.last_success_at.clone()
        },
        running: false,
        ..schedule.clone()
    };
    store_schedule(memory, &next).await?;
    Ok(next)
}

pub(crate) fn resolve_session_id(schedule: &Schedule) -> Option<String> {
    match schedule.session_mode.as_str() {
        "reuse_fixed" => schedule.fixed_session_id.clone(),
        "create_new" => None,
        _ => None,
    }
}

pub(crate) fn next_run_after(from: DateTime<Utc>, interval_minutes: u64) -> String {
    (from + Duration::minutes(interval_minutes as i64)).to_rfc3339()
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn due_delay(next_run_at: &str) -> anyhow::Result<std::time::Duration> {
    let next = DateTime::parse_from_rfc3339(next_run_at)?.with_timezone(&Utc);
    let now = Utc::now();
    if next <= now {
        return Ok(std::time::Duration::from_secs(0));
    }
    let delta = (next - now)
        .to_std()
        .map_err(|_| anyhow::anyhow!("next_run_at must be in the future"))?;
    Ok(delta)
}

async fn store_schedule(memory: &Arc<dyn Memory>, schedule: &Schedule) -> anyhow::Result<()> {
    let payload = serde_json::to_string(&StoredSchedule::from_schedule(schedule))?;
    memory
        .store(
            &schedule_key(&schedule.id),
            &payload,
            MemoryCategory::Core,
            None,
        )
        .await
}

fn schedule_key(schedule_id: &str) -> String {
    format!("{SCHEDULES_PREFIX}{schedule_id}")
}

fn now_text() -> String {
    Utc::now().to_rfc3339()
}

fn schedule_run_succeeded(run: &Run) -> bool {
    run.status == "completed"
}

impl StoredSchedule {
    fn from_schedule(schedule: &Schedule) -> Self {
        Self {
            id: schedule.id.clone(),
            name: schedule.name.clone(),
            agent_id: schedule.agent_id.clone(),
            prompt: schedule.prompt.clone(),
            interval_minutes: schedule.interval_minutes,
            session_mode: schedule.session_mode.clone(),
            fixed_session_id: schedule.fixed_session_id.clone(),
            enabled: schedule.enabled,
            created_at: schedule.created_at.clone(),
            updated_at: schedule.updated_at.clone(),
            next_run_at: schedule.next_run_at.clone(),
            last_started_at: schedule.last_started_at.clone(),
            last_finished_at: schedule.last_finished_at.clone(),
            last_run_id: schedule.last_run_id.clone(),
            last_error: schedule.last_error.clone(),
            consecutive_failure_count: schedule.consecutive_failure_count,
            last_success_at: schedule.last_success_at.clone(),
            running: schedule.running,
        }
    }

    fn into_schedule(self) -> Schedule {
        Schedule {
            id: self.id,
            name: self.name,
            agent_id: self.agent_id,
            prompt: self.prompt,
            interval_minutes: self.interval_minutes,
            session_mode: self.session_mode,
            fixed_session_id: self.fixed_session_id,
            enabled: self.enabled,
            created_at: self.created_at,
            updated_at: self.updated_at,
            next_run_at: self.next_run_at,
            last_started_at: self.last_started_at,
            last_finished_at: self.last_finished_at,
            last_run_id: self.last_run_id,
            last_error: self.last_error,
            consecutive_failure_count: self.consecutive_failure_count,
            last_success_at: self.last_success_at,
            running: self.running,
        }
    }
}

impl From<ScheduleDraft> for Schedule {
    fn from(value: ScheduleDraft) -> Self {
        let now = now_text();
        let next_run_at = if value.enabled {
            Some(next_run_after(Utc::now(), value.interval_minutes))
        } else {
            None
        };
        Self {
            id: value.id,
            name: value.name,
            agent_id: value.agent_id,
            prompt: value.prompt,
            interval_minutes: value.interval_minutes,
            session_mode: value.session_mode,
            fixed_session_id: value.fixed_session_id,
            enabled: value.enabled,
            created_at: now.clone(),
            updated_at: now,
            next_run_at,
            last_started_at: None,
            last_finished_at: None,
            last_run_id: None,
            last_error: None,
            consecutive_failure_count: 0,
            last_success_at: None,
            running: false,
        }
    }
}
