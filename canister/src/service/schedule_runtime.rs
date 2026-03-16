//! where: iclaw/canister/src/service/schedule_runtime.rs
//! what: Wasm timer registration for durable schedules
//! why: keep ic-cdk-timers and lifecycle-specific concerns out of service orchestration

use crate::types::Schedule;

#[cfg(target_arch = "wasm32")]
use ic_cdk_timers::{clear_timer, set_timer, TimerId};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;
#[cfg(target_arch = "wasm32")]
use std::mem;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static REGISTERED_TIMERS: RefCell<BTreeMap<String, TimerId>> = const { RefCell::new(BTreeMap::new()) };
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn clear_registered_timers() {
    REGISTERED_TIMERS.with(|timers| {
        for (_, timer_id) in mem::take(&mut *timers.borrow_mut()) {
            clear_timer(timer_id);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn clear_registered_timers() {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn register_schedule(schedule: &Schedule) {
    let Some(next_run_at) = schedule.next_run_at.as_deref() else {
        unregister_schedule(&schedule.id);
        return;
    };
    let Ok(delay) = super::schedules::due_delay(next_run_at) else {
        return;
    };
    let schedule_id = schedule.id.clone();
    unregister_schedule(&schedule_id);
    let timer_id = set_timer(delay, async move {
        super::spawn_schedule_fire(schedule_id.clone());
    });
    REGISTERED_TIMERS.with(|timers| {
        timers.borrow_mut().insert(schedule.id.clone(), timer_id);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn register_schedule(_schedule: &Schedule) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn unregister_schedule(schedule_id: &str) {
    REGISTERED_TIMERS.with(|timers| {
        if let Some(timer_id) = timers.borrow_mut().remove(schedule_id) {
            clear_timer(timer_id);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn unregister_schedule(_schedule_id: &str) {}

#[cfg(target_arch = "wasm32")]
pub(crate) fn spawn_restore() {
    ic_cdk::futures::spawn(async {
        super::restore_schedule_timers().await;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_restore() {}
