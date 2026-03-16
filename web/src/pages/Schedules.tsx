// where: iclaw/web/src/pages/Schedules.tsx
// what: Operator page for timer-based schedule creation, review, and manual execution
// why: Dense schedule state is easier to operate when list, draft, and detail panels are clearly separated

import { useEffect, useState } from "react";
import type { Agent, Schedule } from "@/generated/iclaw.did";
import { Badge, Card } from "@/components/ui/Card";
import { MetricTile } from "@/components/ui/Field";
import { isFailingSchedule, isStaleSchedule } from "@/lib/schedule-health";
import {
  CreateSchedulePanel,
  ScheduleDetailPanel,
} from "@/pages/schedules/ScheduleEditorPanels";
import type { AsyncActionState, SchedulesViewModel } from "@/types/ui";

function successLabel(schedule: Schedule): string {
  if (schedule.last_success_at[0]) {
    return schedule.last_success_at[0];
  }
  if (schedule.consecutive_failure_count > 0n) {
    return "まだ成功なし";
  }
  return "never";
}

function statusTone(schedule: Schedule): "neutral" | "good" | "warn" | "info" {
  if (isFailingSchedule(schedule) || isStaleSchedule(schedule)) {
    return "warn";
  }
  if (schedule.running) {
    return "info";
  }
  return schedule.enabled ? "good" : "neutral";
}

export function SchedulesPage({
  schedules,
  agents,
  action,
  onCreate,
  onSelectSchedule,
  onUpdate,
  onToggleEnabled,
  onTrigger,
}: {
  schedules: SchedulesViewModel;
  agents: Agent[];
  action: AsyncActionState;
  onCreate: (draft: {
    id: string;
    name: string;
    agentId: string;
    prompt: string;
    intervalMinutes: bigint;
    sessionMode: string;
    fixedSessionId?: string;
    enabled: boolean;
  }) => Promise<void>;
  onSelectSchedule: (scheduleId: string) => void;
  onUpdate: (schedule: Schedule) => Promise<void>;
  onToggleEnabled: (schedule: Schedule) => Promise<void>;
  onTrigger: (scheduleId: string) => Promise<void>;
}) {
  const [filter, setFilter] = useState<"all" | "failing">("all");
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [agentId, setAgentId] = useState("default");
  const [prompt, setPrompt] = useState("");
  const [intervalMinutes, setIntervalMinutes] = useState("60");
  const [sessionMode, setSessionMode] = useState("create_new");
  const [fixedSessionId, setFixedSessionId] = useState("");
  const [editName, setEditName] = useState("");
  const [editAgentId, setEditAgentId] = useState("default");
  const [editPrompt, setEditPrompt] = useState("");
  const [editIntervalMinutes, setEditIntervalMinutes] = useState("60");
  const [editSessionMode, setEditSessionMode] = useState("create_new");
  const [editFixedSessionId, setEditFixedSessionId] = useState("");

  const visibleSchedules =
    filter === "failing"
      ? schedules.items.filter((schedule) => isFailingSchedule(schedule) || isStaleSchedule(schedule))
      : schedules.items;
  const visibleSelectedSchedule =
    visibleSchedules.find((schedule) => schedule.id === schedules.selectedSchedule?.id) ?? null;
  const failingCount = schedules.items.filter(isFailingSchedule).length;
  const staleCount = schedules.items.filter(isStaleSchedule).length;
  const runningCount = schedules.items.filter((schedule) => schedule.running).length;

  useEffect(() => {
    const selected = visibleSelectedSchedule;
    if (!selected) {
      setEditName("");
      setEditAgentId("default");
      setEditPrompt("");
      setEditIntervalMinutes("60");
      setEditSessionMode("create_new");
      setEditFixedSessionId("");
      return;
    }
    setEditName(selected.name);
    setEditAgentId(selected.agent_id);
    setEditPrompt(selected.prompt);
    setEditIntervalMinutes(selected.interval_minutes.toString());
    setEditSessionMode(selected.session_mode);
    setEditFixedSessionId(selected.fixed_session_id[0] ?? "");
  }, [visibleSelectedSchedule]);

  const submit = async () => {
    const parsed = BigInt(intervalMinutes || "0");
    if (!id.trim() || !name.trim() || !prompt.trim() || parsed <= 0n) {
      return;
    }
    await onCreate({
      id: id.trim(),
      name: name.trim(),
      agentId,
      prompt: prompt.trim(),
      intervalMinutes: parsed,
      sessionMode,
      fixedSessionId: sessionMode === "reuse_fixed" ? fixedSessionId.trim() : undefined,
      enabled: true,
    });
    setId("");
    setName("");
    setPrompt("");
    setIntervalMinutes("60");
    setFixedSessionId("");
  };

  const submitUpdate = async () => {
    const selected = visibleSelectedSchedule;
    const parsed = BigInt(editIntervalMinutes || "0");
    if (!selected || !editName.trim() || !editPrompt.trim() || parsed <= 0n) {
      return;
    }
    await onUpdate({
      ...selected,
      name: editName.trim(),
      agent_id: editAgentId,
      prompt: editPrompt.trim(),
      interval_minutes: parsed,
      session_mode: editSessionMode,
      fixed_session_id:
        editSessionMode === "reuse_fixed" && editFixedSessionId.trim()
          ? [editFixedSessionId.trim()]
          : [],
    });
  };

  return (
    <div className="grid gap-6 xl:grid-cols-[0.92fr_1.08fr]">
      <Card
        title="Schedules"
        subtitle="timer ベース automation entry の一覧"
        actions={
          <div className="inline-flex rounded-full border border-zinc-200 bg-zinc-50/80 p-1">
            {(["all", "failing"] as const).map((option) => (
              <button
                key={option}
                type="button"
                onClick={() => setFilter(option)}
                className={`rounded-full px-3 py-1.5 text-xs font-medium transition-colors ${
                  filter === option
                    ? option === "failing"
                      ? "bg-amber-100 text-amber-700"
                      : "bg-sky-100 text-sky-700"
                    : "text-zinc-500"
                }`}
              >
                {option}
              </button>
            ))}
          </div>
        }
      >
        <div className="space-y-4">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricTile label="visible" value={String(visibleSchedules.length)} detail="current filter" tone="info" />
            <MetricTile label="failing" value={String(failingCount)} detail="consecutive failures" tone={failingCount > 0 ? "warn" : "neutral"} />
            <MetricTile label="stale" value={String(staleCount)} detail="missed cadence" tone={staleCount > 0 ? "warn" : "neutral"} />
            <MetricTile label="running" value={String(runningCount)} detail="active timers" tone={runningCount > 0 ? "good" : "neutral"} />
          </div>

          <div className="space-y-3">
            {visibleSchedules.map((schedule) => (
              <button
                key={schedule.id}
                type="button"
                onClick={() => onSelectSchedule(schedule.id)}
                className={`block w-full rounded-3xl border px-4 py-4 text-left transition-colors ${
                  visibleSelectedSchedule?.id === schedule.id
                    ? "border-sky-300 bg-sky-50 shadow-[0_14px_40px_rgba(125,211,252,0.2)]"
                    : schedule.consecutive_failure_count > 0n
                      ? "border-amber-200 bg-amber-50 hover:bg-amber-100/70"
                      : "border-zinc-200 bg-zinc-50/70 hover:bg-white"
                }`}
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-base font-semibold text-zinc-900">{schedule.name}</p>
                    <p className="mt-1 text-xs uppercase tracking-[0.18em] text-zinc-500">
                      {schedule.id}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Badge tone={statusTone(schedule)}>
                      {schedule.running ? "running" : schedule.enabled ? "enabled" : "disabled"}
                    </Badge>
                    <Badge tone="info">{schedule.interval_minutes.toString()}m</Badge>
                    <Badge tone={schedule.consecutive_failure_count > 0n ? "warn" : "good"}>
                      failures {schedule.consecutive_failure_count.toString()}
                    </Badge>
                    {isStaleSchedule(schedule) && <Badge tone="warn">stale</Badge>}
                  </div>
                </div>
                <div className="mt-4 grid gap-3 text-xs text-zinc-600 sm:grid-cols-3">
                  <div>
                    <p className="uppercase tracking-[0.18em] text-zinc-500">next</p>
                    <p className="mt-1">next {schedule.next_run_at[0] ?? "disabled"}</p>
                  </div>
                  <div>
                    <p className="uppercase tracking-[0.18em] text-zinc-500">last success</p>
                    <p className="mt-1">last success {successLabel(schedule)}</p>
                  </div>
                  <div>
                    <p className="uppercase tracking-[0.18em] text-zinc-500">last error</p>
                    <p className="mt-1 line-clamp-2">{schedule.last_error[0] ?? "last_error none"}</p>
                  </div>
                </div>
              </button>
            ))}
            {visibleSchedules.length === 0 && (
              <p className="rounded-2xl border border-dashed border-zinc-200 bg-zinc-50/70 px-4 py-5 text-sm text-zinc-500">
                {filter === "failing" ? "失敗中または stale な schedule はありません。" : "schedule はまだありません。"}
              </p>
            )}
          </div>
        </div>
      </Card>

      <div className="space-y-6">
        <CreateSchedulePanel
          agents={agents}
          action={action}
          draft={{ id, name, agentId, prompt, intervalMinutes, sessionMode, fixedSessionId }}
          onDraftChange={{
            setId,
            setName,
            setAgentId,
            setPrompt,
            setIntervalMinutes,
            setSessionMode,
            setFixedSessionId,
          }}
          onSubmit={submit}
        />

        <ScheduleDetailPanel
          schedule={visibleSelectedSchedule}
          latestRunId={
            visibleSelectedSchedule ? (schedules.latestRuns[visibleSelectedSchedule.id]?.id ?? null) : null
          }
          agents={agents}
          action={action}
          successLabel={successLabel}
          edit={{
            name: editName,
            agentId: editAgentId,
            prompt: editPrompt,
            intervalMinutes: editIntervalMinutes,
            sessionMode: editSessionMode,
            fixedSessionId: editFixedSessionId,
          }}
          onEditChange={{
            setName: setEditName,
            setAgentId: setEditAgentId,
            setPrompt: setEditPrompt,
            setIntervalMinutes: setEditIntervalMinutes,
            setSessionMode: setEditSessionMode,
            setFixedSessionId: setEditFixedSessionId,
          }}
          onUpdate={submitUpdate}
          onToggleEnabled={onToggleEnabled}
          onTrigger={onTrigger}
        />
      </div>
    </div>
  );
}
