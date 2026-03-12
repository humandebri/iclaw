// where: iclaw/web/src/lib/schedule-health.ts
// what: Shared stale/failing schedule predicates for dashboard and schedule pages
// why: Keep operator-facing health rules in one place so the UI does not drift

import type { Schedule } from "@/generated/iclaw.did";

const STALE_GRACE_MULTIPLIER = 2;

function parseTimestamp(value: string | null): number | null {
  if (!value) {
    return null;
  }
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? null : parsed;
}

export function isFailingSchedule(schedule: Schedule): boolean {
  return schedule.consecutive_failure_count > 0n;
}

export function isStaleSchedule(schedule: Schedule, nowMs = Date.now()): boolean {
  if (!schedule.enabled || schedule.running) {
    return false;
  }
  const nextRunAt = parseTimestamp(schedule.next_run_at[0] ?? null);
  if (nextRunAt !== null && nextRunAt < nowMs) {
    return true;
  }
  if (!isFailingSchedule(schedule)) {
    return false;
  }
  const lastSuccessAt = parseTimestamp(schedule.last_success_at[0] ?? null);
  if (lastSuccessAt === null) {
    return true;
  }
  const intervalMs = Number(schedule.interval_minutes) * 60_000;
  return lastSuccessAt + intervalMs * STALE_GRACE_MULTIPLIER < nowMs;
}
