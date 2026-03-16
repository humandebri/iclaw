// where: iclaw/web/src/pages/schedules/ScheduleEditorPanels.tsx
// what: Presentational panels for schedule creation and detail editing
// why: Keep the main schedule page focused on state and selection while preserving a denser UI

import type { Agent, Schedule } from "@/generated/iclaw.did";
import { Badge, Card } from "@/components/ui/Card";
import { Field, MetricTile } from "@/components/ui/Field";
import { isStaleSchedule } from "@/lib/schedule-health";
import type { AsyncActionState } from "@/types/ui";

function inputClassName() {
  return "w-full rounded-2xl border border-zinc-200 bg-zinc-50/88 px-4 py-3 text-sm text-zinc-900 shadow-sm shadow-zinc-200/40 outline-none transition-colors focus:border-sky-300 focus:bg-white";
}

export function CreateSchedulePanel({
  agents,
  action,
  draft,
  onDraftChange,
  onSubmit,
}: {
  agents: Agent[];
  action: AsyncActionState;
  draft: {
    id: string;
    name: string;
    agentId: string;
    prompt: string;
    intervalMinutes: string;
    sessionMode: string;
    fixedSessionId: string;
  };
  onDraftChange: {
    setId: (value: string) => void;
    setName: (value: string) => void;
    setAgentId: (value: string) => void;
    setPrompt: (value: string) => void;
    setIntervalMinutes: (value: string) => void;
    setSessionMode: (value: string) => void;
    setFixedSessionId: (value: string) => void;
  };
  onSubmit: () => Promise<void>;
}) {
  return (
    <Card title="Create Schedule" subtitle="canister timer で定期実行する run を追加">
      <div className="rounded-2xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-700">
        固定 prompt と固定 interval だけを持つ、最小構成の schedule editor です。
      </div>
      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <Field label="schedule id">
          <input value={draft.id} onChange={(event) => onDraftChange.setId(event.target.value)} placeholder="schedule id" className={inputClassName()} />
        </Field>
        <Field label="display name">
          <input value={draft.name} onChange={(event) => onDraftChange.setName(event.target.value)} placeholder="display name" className={inputClassName()} />
        </Field>
        <Field label="agent">
          <select value={draft.agentId} onChange={(event) => onDraftChange.setAgentId(event.target.value)} className={inputClassName()}>
            {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
          </select>
        </Field>
        <Field label="interval minutes" hint="1 minute 以上の fixed interval">
          <input value={draft.intervalMinutes} onChange={(event) => onDraftChange.setIntervalMinutes(event.target.value)} placeholder="interval minutes" className={inputClassName()} />
        </Field>
        <Field label="session mode">
          <select value={draft.sessionMode} onChange={(event) => onDraftChange.setSessionMode(event.target.value)} className={inputClassName()}>
            <option value="create_new">create_new</option>
            <option value="reuse_fixed">reuse_fixed</option>
          </select>
        </Field>
        {draft.sessionMode === "reuse_fixed" && (
          <Field label="fixed session id">
            <input value={draft.fixedSessionId} onChange={(event) => onDraftChange.setFixedSessionId(event.target.value)} placeholder="fixed session id" className={inputClassName()} />
          </Field>
        )}
        <div className="md:col-span-2">
          <Field label="fixed prompt">
            <textarea value={draft.prompt} onChange={(event) => onDraftChange.setPrompt(event.target.value)} placeholder="fixed prompt" rows={5} className={inputClassName()} />
          </Field>
        </div>
      </div>
      {action.error && <p className="mt-4 text-sm text-rose-700">{action.error}</p>}
      {action.successMessage && <p className="mt-4 text-sm text-emerald-700">{action.successMessage}</p>}
      <button type="button" onClick={() => void onSubmit()} disabled={action.pending} className="mt-5 rounded-2xl bg-blue-600 px-5 py-3 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:bg-blue-900/60">
        {action.pending ? "Saving" : "Create schedule"}
      </button>
    </Card>
  );
}

export function ScheduleDetailPanel({
  schedule,
  latestRunId,
  agents,
  action,
  successLabel,
  edit,
  onEditChange,
  onUpdate,
  onToggleEnabled,
  onTrigger,
}: {
  schedule: Schedule | null;
  latestRunId: string | null;
  agents: Agent[];
  action: AsyncActionState;
  successLabel: (schedule: Schedule) => string;
  edit: {
    name: string;
    agentId: string;
    prompt: string;
    intervalMinutes: string;
    sessionMode: string;
    fixedSessionId: string;
  };
  onEditChange: {
    setName: (value: string) => void;
    setAgentId: (value: string) => void;
    setPrompt: (value: string) => void;
    setIntervalMinutes: (value: string) => void;
    setSessionMode: (value: string) => void;
    setFixedSessionId: (value: string) => void;
  };
  onUpdate: () => Promise<void>;
  onToggleEnabled: (schedule: Schedule) => Promise<void>;
  onTrigger: (scheduleId: string) => Promise<void>;
}) {
  return (
    <Card title="Schedule Detail" subtitle="実行状態と編集">
      {!schedule && <p className="text-sm text-zinc-500">schedule を選ぶと詳細が見えます。</p>}
      {schedule && (
        <div className="space-y-5 text-sm text-zinc-600">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricTile label="runtime" value={schedule.running ? "running" : "idle"} detail={`agent ${schedule.agent_id}`} tone={schedule.running ? "info" : "neutral"} />
            <MetricTile label="next" value={schedule.next_run_at[0] ?? "disabled"} detail={`failures ${schedule.consecutive_failure_count.toString()}`} tone={schedule.consecutive_failure_count > 0n ? "warn" : "info"} />
            <MetricTile label="last success" value={successLabel(schedule)} detail="operator recovery baseline" tone={schedule.consecutive_failure_count > 0n ? "warn" : "good"} />
            <MetricTile label="last finished" value={schedule.last_finished_at[0] ?? "never"} detail={isStaleSchedule(schedule) ? "stale cadence" : schedule.enabled ? "timer enabled" : "timer disabled"} tone={isStaleSchedule(schedule) ? "warn" : schedule.enabled ? "good" : "neutral"} />
          </div>

          <div className="rounded-3xl border border-zinc-200 bg-zinc-50/70 p-4">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Latest Run</p>
                <p className="mt-2 text-base font-medium text-zinc-900">{latestRunId ?? "none"}</p>
              </div>
              <Badge tone={schedule.consecutive_failure_count > 0n ? "warn" : "info"}>
                {schedule.session_mode}
              </Badge>
            </div>
            <p className="mt-3 text-xs text-zinc-500">last success {successLabel(schedule)}</p>
            <p className="mt-2 text-xs text-zinc-500">last finished {schedule.last_finished_at[0] ?? "never"}</p>
            <p className="mt-2 text-xs text-zinc-500">{isStaleSchedule(schedule) ? "stale: next_run_at or recovery window exceeded" : "cadence is within expected window"}</p>
            <p className="mt-2 text-xs text-zinc-500">{schedule.last_error[0] ?? "no last_error"}</p>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Field label="display name">
              <input value={edit.name} onChange={(event) => onEditChange.setName(event.target.value)} placeholder="display name" className={inputClassName()} />
            </Field>
            <Field label="agent">
              <select value={edit.agentId} onChange={(event) => onEditChange.setAgentId(event.target.value)} className={inputClassName()}>
                {agents.map((agent) => <option key={agent.id} value={agent.id}>{agent.name}</option>)}
              </select>
            </Field>
            <Field label="interval minutes">
              <input value={edit.intervalMinutes} onChange={(event) => onEditChange.setIntervalMinutes(event.target.value)} placeholder="interval minutes" className={inputClassName()} />
            </Field>
            <Field label="session mode">
              <select value={edit.sessionMode} onChange={(event) => onEditChange.setSessionMode(event.target.value)} className={inputClassName()}>
                <option value="create_new">create_new</option>
                <option value="reuse_fixed">reuse_fixed</option>
              </select>
            </Field>
            {edit.sessionMode === "reuse_fixed" && (
              <Field label="fixed session id">
                <input value={edit.fixedSessionId} onChange={(event) => onEditChange.setFixedSessionId(event.target.value)} placeholder="fixed session id" className={inputClassName()} />
              </Field>
            )}
            <div className="md:col-span-2">
              <Field label="fixed prompt">
                <textarea value={edit.prompt} onChange={(event) => onEditChange.setPrompt(event.target.value)} placeholder="fixed prompt" rows={5} className={inputClassName()} />
              </Field>
            </div>
          </div>

          <p className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
            disabled は timer の自動実行だけを止めます。operator の Trigger now は引き続き使えます。
          </p>

          <div className="flex flex-wrap gap-3">
            <button type="button" onClick={() => void onUpdate()} disabled={action.pending} className="rounded-2xl bg-zinc-900 px-5 py-3 text-sm font-medium text-white transition-colors hover:bg-zinc-800 disabled:bg-zinc-300 disabled:text-zinc-500">
              {action.pending ? "Saving" : "Save detail changes"}
            </button>
            <button type="button" onClick={() => void onToggleEnabled(schedule)} className="rounded-2xl border border-zinc-200 px-5 py-3 text-sm text-zinc-800 transition-colors hover:bg-zinc-50">
              {schedule.enabled ? "Disable schedule" : "Enable schedule"}
            </button>
            <button type="button" onClick={() => void onTrigger(schedule.id)} className="rounded-2xl border border-emerald-200 px-5 py-3 text-sm text-emerald-700 transition-colors hover:bg-emerald-50">
              Trigger now
            </button>
          </div>
        </div>
      )}
    </Card>
  );
}
