// where: iclaw/web/src/pages/Runs.tsx
// what: Read-only plus minimal control plane view for session runs and events
// why: Operators need one screen to inspect completed and failed runs before deeper tooling exists

import { useEffect } from "react";
import { useSearchParams } from "react-router-dom";
import { Badge, Card } from "@/components/ui/Card";
import type { Run, RunEvent } from "@/generated/iclaw.did";
import type { RunsViewModel } from "@/types/ui";

const softPanelClassName = "rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4";
const softCardClassName = "rounded-xl border border-zinc-200 bg-white/70 px-3 py-3";

export function RunsPage({
  sessionId,
  runs,
  onRefresh,
  onSelectRun,
  onCancelRun,
  onResumeRun,
}: {
  sessionId: string;
  runs: RunsViewModel;
  onRefresh: () => Promise<void>;
  onSelectRun: (runId: string) => Promise<void>;
  onCancelRun: (runId: string) => Promise<void>;
  onResumeRun: (runId: string) => Promise<void>;
}) {
  const [searchParams, setSearchParams] = useSearchParams();
  const requestedRunId = searchParams.get("runId");
  const selectedRun = runs.selectedRun;
  const selectedRunEvents = selectedRun ? runs.events.filter((event) => event.run_id === selectedRun.id) : [];
  const showApprovalFlow =
    selectedRun?.status === "blocked" ||
    selectedRun?.pending_tool_calls.length ||
    selectedRunEvents.some((event) => event.kind === "approved" || event.kind === "resumed");
  const resolvedRunId =
    runs.items.find((run) => run.id === requestedRunId)?.id ??
    selectedRun?.id ??
    runs.items[0]?.id ??
    null;

  useEffect(() => {
    if (requestedRunId && selectedRun?.id !== requestedRunId && runs.items.some((run) => run.id === requestedRunId)) {
      void onSelectRun(requestedRunId);
      return;
    }
    if (resolvedRunId && requestedRunId !== resolvedRunId) {
      setSearchParams({ runId: resolvedRunId }, { replace: true });
    }
  }, [onSelectRun, requestedRunId, resolvedRunId, runs.items, selectedRun?.id, setSearchParams]);

  const handleRunSelect = (runId: string) => {
    setSearchParams({ runId }, { replace: false });
    void onSelectRun(runId);
  };

  const handleRefresh = () => {
    void onRefresh();
  };

  return (
    <div className="space-y-6">
      <Card
        title="Runs"
        subtitle="現在 session の run 一覧と event の read-only 面"
        actions={
          <button
            type="button"
            onClick={handleRefresh}
            className="rounded-xl border border-zinc-200 px-4 py-2 text-sm text-zinc-800 transition-colors hover:bg-zinc-50"
          >
            Refresh
          </button>
        }
      >
        <div className="flex flex-wrap gap-2">
          <Badge tone="info">{sessionId ? `session ${sessionId}` : "session not selected"}</Badge>
          <Badge tone={runs.items.length > 0 ? "good" : "neutral"}>{runs.items.length} runs</Badge>
        </div>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <Card title="Run List">
          <div className="space-y-3">
            {runs.items.length === 0 && <p className="text-sm text-zinc-500">run はまだありません。</p>}
            {runs.items.map((run) => (
              <button
                key={run.id}
                type="button"
                onClick={() => handleRunSelect(run.id)}
                className={`block w-full rounded-2xl border px-4 py-3 text-left transition-colors ${
                  runs.selectedRun?.id === run.id
                    ? "border-sky-300 bg-sky-50"
                    : "border-zinc-200 bg-zinc-50/70 hover:bg-white"
                }`}
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-sm font-medium text-zinc-900">{run.id}</p>
                    <p className="mt-2 line-clamp-2 text-sm text-zinc-600">{run.prompt}</p>
                  </div>
                  <Badge tone={toneForStatus(run.status)}>{run.status}</Badge>
                </div>
                <p className="mt-3 text-xs text-zinc-500">{new Date(run.created_at).toLocaleString()}</p>
              </button>
            ))}
          </div>
        </Card>

        <div className="space-y-6">
          <Card
            title="Run Detail"
            actions={
              selectedRun ? (
                <div className="flex flex-wrap gap-2">
                  {(selectedRun.status === "queued" || selectedRun.status === "running") && (
                    <button
                      type="button"
                      onClick={() => void onCancelRun(selectedRun.id)}
                      className="rounded-xl border border-rose-200 px-4 py-2 text-sm text-rose-700 transition-colors hover:bg-rose-50"
                    >
                      Mark Cancelled
                    </button>
                  )}
                  {selectedRun.status === "blocked" && selectedRun.pending_tool_calls.length > 0 && (
                    <button
                      type="button"
                      onClick={() => void onResumeRun(selectedRun.id)}
                      className="rounded-xl border border-emerald-200 px-4 py-2 text-sm text-emerald-700 transition-colors hover:bg-emerald-50"
                    >
                      Resume run
                    </button>
                  )}
                </div>
              ) : undefined
            }
          >
            {!selectedRun && <p className="text-sm text-zinc-500">run を選ぶと詳細が見えます。</p>}
            {selectedRun && (
              <div className="space-y-4 text-sm text-zinc-600">
                <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-700">
                  cancel は実行中 update call を強制停止しません。run の状態と event を cancelled に記録するだけです。
                </div>
                <div className="flex flex-wrap gap-2">
                  <Badge tone={toneForStatus(selectedRun.status)}>{selectedRun.status}</Badge>
                  <Badge tone="info">agent {selectedRun.agent_id}</Badge>
                  <Badge tone={selectedRun.provider_ready ? "good" : "warn"}>
                    provider {selectedRun.provider_ready ? "ready" : "degraded"}
                  </Badge>
                  <Badge tone={selectedRun.memory_ready ? "good" : "warn"}>
                    memory {selectedRun.memory_ready ? "ready" : "degraded"}
                  </Badge>
                </div>
                <Detail label="Prompt" value={selectedRun.prompt} />
                <Detail label="Response" value={selectedRun.response[0] ?? "no response"} />
                <Detail label="Error" value={selectedRun.error[0] ?? "none"} />
                {selectedRun.status === "blocked" && (
                  <Detail
                    label="Pending Assistant"
                    value={selectedRun.pending_assistant_text[0] ?? "no pending assistant text"}
                  />
                )}
                {selectedRun.status === "blocked" && selectedRun.pending_tool_calls.length > 0 && (
                  <div className={softPanelClassName}>
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Pending Tool Calls</p>
                      <Badge tone="warn">approval required</Badge>
                    </div>
                    <div className="mt-3 space-y-3">
                      {selectedRun.pending_tool_calls.map((call) => (
                        <div key={call.id} className={softCardClassName}>
                          <p className="text-sm font-medium text-zinc-900">{call.name}</p>
                          <p className="mt-1 text-xs text-zinc-500">{call.id}</p>
                          <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">{call.arguments}</p>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}
          </Card>

          {selectedRun && showApprovalFlow && (
            <Card title="Approval Flow" subtitle="blocked から resumed までの監査メモ">
              <ApprovalFlow run={selectedRun} events={selectedRunEvents} />
            </Card>
          )}

          <Card title="Run Events">
            <div className="space-y-3">
              {runs.events.length === 0 && <p className="text-sm text-zinc-500">event はまだありません。</p>}
              {runs.events.map((event) => (
                <div key={event.id} className="rounded-2xl border border-zinc-200 bg-zinc-50/70 px-4 py-3">
                  <div className="flex items-center justify-between gap-3">
                    <Badge tone={toneForEvent(event.kind)}>{event.kind}</Badge>
                    <p className="text-xs text-zinc-500">{new Date(event.timestamp).toLocaleString()}</p>
                  </div>
                  <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">{event.message}</p>
                </div>
              ))}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
      <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">{label}</p>
      <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">{value}</p>
    </div>
  );
}

function ApprovalFlow({ run, events }: { run: Run; events: RunEvent[] }) {
  const approved = events.find((event) => event.kind === "approved") ?? null;
  const resumed = events.find((event) => event.kind === "resumed") ?? null;
  const latestToolSuccess = [...events].reverse().find((event) => event.kind === "tool_succeeded") ?? null;
  const latestToolFailure = [...events].reverse().find((event) => event.kind === "tool_failed") ?? null;
  const latestOutcome = latestToolFailure ?? latestToolSuccess;

  return (
    <div className="space-y-4 text-sm text-zinc-600">
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <AuditTile
          label="Pending Assistant"
          value={run.pending_assistant_text[0] ?? "no pending assistant text"}
          tone={run.pending_assistant_text[0] ? "warn" : "neutral"}
        />
        <AuditTile
          label="Pending Tool"
          value={run.pending_tool_calls[0]?.name ?? "none"}
          tone={run.pending_tool_calls.length > 0 ? "warn" : "neutral"}
        />
        <AuditTile
          label="Approved"
          value={approved ? approved.timestamp : "not yet"}
          tone={approved ? "good" : "warn"}
        />
        <AuditTile
          label="Resumed"
          value={resumed ? resumed.timestamp : run.status === "blocked" ? "waiting" : "completed without resume"}
          tone={resumed ? "good" : run.status === "blocked" ? "warn" : "neutral"}
        />
      </div>

      {run.pending_tool_calls.length > 0 && (
        <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
          <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Pending Tool Arguments</p>
          <div className="mt-3 space-y-3">
            {run.pending_tool_calls.map((call) => (
              <div key={call.id} className={softCardClassName}>
                <div className="flex items-center justify-between gap-3">
                  <p className="text-sm font-medium text-zinc-900">{call.name}</p>
                  <p className="text-xs text-zinc-500">{call.id}</p>
                </div>
                <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">{call.arguments}</p>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Latest Outcome</p>
        <p className="mt-3 text-sm text-zinc-700">
          {latestOutcome ? `${latestOutcome.kind} · ${latestOutcome.message}` : "resume 後の tool outcome はまだありません。"}
        </p>
      </div>
    </div>
  );
}

function AuditTile({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "neutral" | "good" | "warn" | "info";
}) {
  return (
    <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">{label}</p>
        <Badge tone={tone}>{label}</Badge>
      </div>
      <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">{value}</p>
    </div>
  );
}

function toneForStatus(status: string): "neutral" | "good" | "warn" | "info" {
  if (status === "completed") {
    return "good";
  }
  if (status === "failed" || status === "cancelled" || status === "blocked") {
    return "warn";
  }
  if (status === "running") {
    return "info";
  }
  return "neutral";
}

function toneForEvent(kind: string): "neutral" | "good" | "warn" | "info" {
  if (kind === "approved" || kind === "resumed" || kind === "tool_succeeded") {
    return "good";
  }
  if (kind === "blocked" || kind === "tool_failed" || kind === "cancelled") {
    return "warn";
  }
  if (kind === "started" || kind === "queued") {
    return "info";
  }
  return "neutral";
}
