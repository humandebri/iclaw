// where: iclaw/web/src/pages/Dashboard.tsx
// what: Operational snapshot combining health and current-session observation
// why: Operators should see runtime readiness and session behavior on one screen before taking action

import { Link } from "react-router-dom";
import { Activity, ArrowRight, Brain, Workflow } from "lucide-react";
import { AllowlistPanel } from "@/components/access/AllowlistPanel";
import { Badge, Card, StatCard } from "@/components/ui/Card";
import type { Agent, HealthResponse, Run, ToolPolicy } from "@/generated/iclaw.did";
import { BlockedRunsCard } from "@/pages/dashboard/BlockedRunsCard";
import type { ObserveViewModel, ScheduleAlertItem } from "@/types/ui";

function scheduleSuccessLabel(alert: ScheduleAlertItem): string {
  if (alert.lastSuccessAt) {
    return alert.lastSuccessAt;
  }
  if (alert.consecutiveFailureCount > 0n) {
    return "まだ成功なし";
  }
  return "never";
}

export function Dashboard({
  health,
  observe,
  currentAgent,
  latestRun,
  latestScheduleRun,
  latestScheduleFailure,
  failingScheduleCount,
  staleScheduleCount,
  runningScheduleCount,
  blockedRunCount,
  latestBlockedRun,
  scheduleAlerts,
  latestWebhookRun,
  latestWebhookFailure,
  scheduleCount,
  webhookCount,
  runCount,
  toolPolicies,
  allowlist,
  loading,
  error,
  onRefresh,
  onAllowlistRefresh,
  onAllowlistSave,
}: {
  health: HealthResponse | null;
  observe: ObserveViewModel;
  currentAgent: Agent | null;
  latestRun: Run | null;
  latestScheduleRun: Run | null;
  latestScheduleFailure: Run | null;
  failingScheduleCount: number;
  staleScheduleCount: number;
  runningScheduleCount: number;
  blockedRunCount: number;
  latestBlockedRun: Run | null;
  scheduleAlerts: ScheduleAlertItem[];
  latestWebhookRun: Run | null;
  latestWebhookFailure: Run | null;
  scheduleCount: number;
  webhookCount: number;
  runCount: number;
  toolPolicies: ToolPolicy[];
  allowlist: {
    principals: string[];
    currentPrincipal: string;
    pending: boolean;
    error: string | null;
    success: string | null;
  };
  loading: boolean;
  error: string | null;
  onRefresh: () => Promise<void>;
  onAllowlistRefresh: () => Promise<void>;
  onAllowlistSave: (principals: string[]) => Promise<void>;
}) {
  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-3">
          <div className="inline-flex items-center gap-2 rounded-full border border-blue-400/20 bg-blue-500/10 px-3 py-1 text-xs font-medium tracking-[0.18em] text-blue-100">
            <Activity className="h-3.5 w-3.5" />
            Operator View
          </div>
          <div>
            <p className="text-sm uppercase tracking-[0.24em] text-blue-200/60">Operator View</p>
            <h2 className="mt-2 text-3xl font-semibold text-white">Caller UX Dashboard</h2>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">
              canister health、current session observation、schedule / webhook 状態を同じ密度で追える運用ビューです。
            </p>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Badge tone={health?.provider_ready ? "good" : "warn"}>
            provider {health?.provider_ready ? "ready" : "degraded"}
          </Badge>
          <Badge tone={health?.memory_ready ? "good" : "warn"}>
            memory {health?.memory_ready ? "ready" : "degraded"}
          </Badge>
          <button
            type="button"
            onClick={() => void onRefresh()}
            className="rounded-2xl border border-white/10 px-4 py-2 text-sm text-slate-200 transition-colors hover:bg-white/5"
          >
            Refresh
          </button>
        </div>
      </div>

      {error && (
        <div className="rounded-2xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100">
          {error}
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard label="Status" value={health?.status ?? (loading ? "loading" : "unknown")} detail={health?.runtime} />
        <StatCard label="Version" value={health?.version ?? "-"} detail={health?.provider_ready ? "provider ready" : "provider degraded"} />
        <StatCard label="Agent" value={currentAgent?.name ?? "-"} detail={currentAgent?.status ?? "not selected"} />
        <StatCard label="Tools" value={String(currentAgent?.enabled_tool_names.length ?? 0)} detail={`${toolPolicies.length} policy rows`} />
        <StatCard label="Runs" value={String(runCount)} detail={latestRun ? `latest ${latestRun.status}` : "no runs yet"} />
        <StatCard label="Schedules" value={String(scheduleCount)} detail={latestScheduleRun ? `latest ${latestScheduleRun.status}` : "no schedule runs"} />
        <StatCard label="Failing Schedules" value={String(failingScheduleCount)} detail={latestScheduleFailure ? latestScheduleFailure.id : "no active failures"} />
        <StatCard label="Stale Schedules" value={String(staleScheduleCount)} detail={staleScheduleCount > 0 ? "needs operator attention" : "cadence looks healthy"} />
        <StatCard label="Running Schedules" value={String(runningScheduleCount)} detail={runningScheduleCount > 0 ? "check schedule alerts" : "idle"} />
        <StatCard label="Blocked Runs" value={String(blockedRunCount)} detail={latestBlockedRun ? latestBlockedRun.id : "no approvals pending"} />
        <StatCard label="Webhooks" value={String(webhookCount)} detail={latestWebhookRun ? `latest ${latestWebhookRun.status}` : "no webhook runs"} />
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <Card
          title="Current Observation"
          subtitle="health() と agent_observe(current session) の合成ビュー"
          actions={<Badge tone={currentAgent?.requires_tool_approval ? "warn" : "good"}>approval {currentAgent?.requires_tool_approval ? "required" : "not required"}</Badge>}
        >
          <div className="grid gap-4 md:grid-cols-2">
            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <div className="flex items-center gap-2 text-sm text-slate-300">
                <Workflow className="h-4 w-4 text-blue-300" />
                Tool loop
              </div>
              <p className="mt-3 text-2xl font-semibold text-white">
                {observe.observation?.tool_loop_enabled ? "Enabled" : "Disabled"}
              </p>
              <p className="mt-2 text-sm text-slate-400">
                max iterations: {observe.observation?.max_tool_iterations ?? "-"}
              </p>
            </div>

            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <div className="flex items-center gap-2 text-sm text-slate-300">
                <Brain className="h-4 w-4 text-blue-300" />
                Summary
              </div>
              <p className="mt-3 text-2xl font-semibold text-white">
                {observe.observation?.conversation_summary_present ? "Present" : "Missing"}
              </p>
              <p className="mt-2 text-sm text-slate-400">
                {observe.observation?.conversation_summary_key ?? "no summary key"}
              </p>
            </div>
          </div>

          <div className="mt-4 grid gap-4 md:grid-cols-2">
            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Schedule Run</p>
              <p className="mt-3 text-sm text-slate-200">
                {latestScheduleRun ? `${latestScheduleRun.id} · ${latestScheduleRun.status}` : "none"}
              </p>
            </div>
            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Schedule Failure</p>
              <p className="mt-3 text-sm text-slate-200">
                {latestScheduleFailure ? `${latestScheduleFailure.id} · ${latestScheduleFailure.status}` : "none"}
              </p>
            </div>
            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Webhook Run</p>
              <p className="mt-3 text-sm text-slate-200">
                {latestWebhookRun ? `${latestWebhookRun.id} · ${latestWebhookRun.status}` : "none"}
              </p>
            </div>
            <div className="rounded-3xl border border-white/10 bg-slate-950/60 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Webhook Failure</p>
              <p className="mt-3 text-sm text-slate-200">
                {latestWebhookFailure ? `${latestWebhookFailure.id} · ${latestWebhookFailure.status}` : "none"}
              </p>
            </div>
          </div>

          <div className="mt-4 flex flex-wrap gap-2">
            {toolPolicies
              .filter((policy) => currentAgent?.enabled_tool_names.includes(policy.tool_name))
              .map((policy) => (
                <Badge key={policy.tool_name} tone={policy.requires_approval || !policy.enabled ? "warn" : "info"}>
                  {policy.tool_name}
                </Badge>
              ))}
            {(observe.observation?.auto_promoted_keys ?? []).slice(0, 6).map((key) => (
              <Badge key={key} tone="info">{key}</Badge>
            ))}
            {(observe.observation?.auto_promoted_keys ?? []).length === 0 && toolPolicies.length === 0 && (
              <p className="text-sm text-slate-500">auto-promoted keys are empty</p>
            )}
          </div>
        </Card>

        <div className="space-y-6">
          <BlockedRunsCard latestBlockedRun={latestBlockedRun} />

          <Card title="Schedule Alerts" subtitle="失敗中 schedule を優先表示">
            <div className="space-y-3">
              {scheduleAlerts.map((alert) => (
                <div key={alert.id} className="rounded-3xl border border-amber-400/20 bg-amber-500/10 px-4 py-4 text-sm text-amber-50 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <p className="font-medium">{alert.name}</p>
                      <p className="mt-1 text-xs text-amber-100/80">{alert.id}</p>
                    </div>
                    <div className="flex gap-2">
                      <Badge tone="warn">failures {alert.consecutiveFailureCount.toString()}</Badge>
                      {alert.stale && <Badge tone="warn">stale</Badge>}
                      <Badge tone={alert.running ? "info" : alert.enabled ? "warn" : "neutral"}>
                        {alert.running ? "running" : alert.enabled ? "enabled" : "disabled"}
                      </Badge>
                    </div>
                  </div>
                  <div className="mt-3 grid gap-2 text-xs text-amber-100/80 sm:grid-cols-2">
                    <p>last success {scheduleSuccessLabel(alert)}</p>
                    <p>next {alert.nextRunAt ?? "disabled"}</p>
                  </div>
                </div>
              ))}
              {scheduleAlerts.length === 0 && <p className="text-sm text-slate-500">no failing schedules</p>}
            </div>
          </Card>

          <Card title="Quick Actions" subtitle="よく使う導線だけを短く出す">
            <div className="space-y-3">
              {[
                { to: "/chat", label: "Chat へ移動", icon: MessageLabel("prompt and observe") },
                { to: "/runs", label: "Runs を確認", icon: MessageLabel(latestRun?.status ?? "run timeline") },
                { to: "/agents", label: "Agents を確認", icon: MessageLabel(currentAgent?.id ?? "agent control") },
                { to: "/schedules", label: "Schedules を確認", icon: MessageLabel(latestScheduleRun?.id ?? "timer automation") },
                { to: "/webhooks", label: "Webhooks を確認", icon: MessageLabel(latestWebhookRun?.id ?? "automation entry") },
                { to: "/observe", label: "Summary を確認", icon: MessageLabel("conversation summary") },
                { to: "/memory", label: "Core / Seed を編集", icon: MessageLabel("memory operations") },
              ].map(({ to, label, icon }) => (
                <Link
                  key={to}
                  to={to}
                  className="flex items-center justify-between rounded-3xl border border-white/10 bg-slate-950/60 px-4 py-3 text-sm text-slate-200 transition-colors hover:bg-white/5"
                >
                  <span className="flex items-center gap-3">
                    {icon}
                    {label}
                  </span>
                  <ArrowRight className="h-4 w-4" />
                </Link>
              ))}
            </div>
          </Card>

          <AllowlistPanel
            principals={allowlist.principals}
            currentPrincipal={allowlist.currentPrincipal}
            pending={allowlist.pending}
            error={allowlist.error}
            success={allowlist.success}
            onRefresh={onAllowlistRefresh}
            onSave={onAllowlistSave}
          />
        </div>
      </div>
    </div>
  );
}

function MessageLabel(detail: string) {
  return (
    <span className="inline-flex items-center gap-2">
      <Activity className="h-4 w-4 text-blue-300" />
      <span className="text-slate-300/80">{detail}</span>
    </span>
  );
}
