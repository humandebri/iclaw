// where: iclaw/web/src/pages/Dashboard.tsx
// what: Operational snapshot combining health, activity, and approval-focused actions
// why: Operators should scan the dashboard top-down and immediately see what is healthy, busy, or needs action

import { Link } from "react-router-dom";
import { ArrowRight } from "lucide-react";
import { AllowlistPanel } from "@/components/access/AllowlistPanel";
import { Badge, Card, StatCard } from "@/components/ui/Card";
import type { Agent, HealthResponse, Run, ToolPolicy } from "@/generated/iclaw.did";
import { BlockedRunsCard } from "@/pages/dashboard/BlockedRunsCard";
import { DashboardHero } from "@/pages/dashboard/DashboardHero";
import { DashboardSnapshot } from "@/pages/dashboard/DashboardSnapshot";
import type { ObserveViewModel, ScheduleAlertItem } from "@/types/ui";

type MetricTone = "neutral" | "success" | "pending" | "error" | "info";
type MetricCard = {
  label: string;
  value: string;
  detail: string;
  tone: MetricTone;
  emphasis?: "normal" | "strong";
};
type MetricGroup = {
  title: string;
  cards: MetricCard[];
};

function scheduleSuccessLabel(alert: ScheduleAlertItem): string {
  if (alert.lastSuccessAt) {
    return alert.lastSuccessAt;
  }
  if (alert.consecutiveFailureCount > 0n) {
    return "まだ成功なし";
  }
  return "never";
}

function failureTarget(latestScheduleFailure: Run | null, blockedRuns: Run[]): string {
  if (latestScheduleFailure) {
    return `latest ${latestScheduleFailure.id} を確認`;
  }
  if (blockedRuns[0]) {
    return `approval queue ${blockedRuns[0].id}`;
  }
  return "active failure はありません";
}

function metricToneFromCount(count: number, hasHealthyFallback = false): MetricTone {
  if (count > 0) {
    return "pending";
  }
  return hasHealthyFallback ? "success" : "neutral";
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
  blockedRuns,
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
  blockedRuns: Run[];
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
  const metricGroups: MetricGroup[] = [
    {
      title: "Health",
      cards: [
        { label: "Status", value: health?.status ?? (loading ? "loading" : "unknown"), detail: health?.runtime ?? "runtime unavailable", tone: health ? "success" : "pending" },
        { label: "Version", value: health?.version ?? "-", detail: health?.provider_ready ? "provider ready" : "provider degraded", tone: health?.provider_ready ? "success" : "pending" },
        { label: "Agent", value: currentAgent?.name ?? "-", detail: currentAgent?.status ?? "not selected", tone: currentAgent ? "info" : "pending" },
      ],
    },
    {
      title: "Workload",
      cards: [
        { label: "Tools", value: String(currentAgent?.enabled_tool_names.length ?? 0), detail: `${toolPolicies.length} policy rows`, tone: "info" },
        { label: "Runs", value: String(runCount), detail: latestRun ? `latest ${latestRun.status}` : "no runs yet", tone: "neutral" },
        { label: "Schedules", value: String(scheduleCount), detail: `${runningScheduleCount} running · ${failingScheduleCount} failing`, tone: metricToneFromCount(failingScheduleCount) },
      ],
    },
    {
      title: "Resources",
      cards: [
        { label: "Stale Schedules", value: String(staleScheduleCount), detail: staleScheduleCount > 0 ? "cadence review が必要" : "cadence looks healthy", tone: metricToneFromCount(staleScheduleCount, true) },
        { label: "Blocked Runs", value: String(blockedRunCount), detail: blockedRuns[0] ? `next ${blockedRuns[0].id}` : "no approvals pending", tone: blockedRunCount > 0 ? "error" : "success", emphasis: "strong" },
        { label: "Webhooks", value: String(webhookCount), detail: latestWebhookRun ? `latest ${latestWebhookRun.status}` : "no webhook runs", tone: latestWebhookFailure ? "pending" : "neutral" },
      ],
    },
  ];

  return (
    <div className="space-y-6">
      <DashboardHero
        health={health}
        currentAgent={currentAgent}
        blockedRuns={blockedRuns}
        latestScheduleFailure={latestScheduleFailure}
        latestWebhookFailure={latestWebhookFailure}
        runCount={runCount}
        blockedRunCount={blockedRunCount}
        onRefresh={onRefresh}
      />

      {error && (
        <div className="rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
          {error}
        </div>
      )}

      <div className="grid gap-4 xl:grid-cols-3">
        {metricGroups.map((group) => (
          <Card key={group.title} title={group.title} variant="light">
            <div className="grid gap-3">
              {group.cards.map((card) => (
                <StatCard
                  key={card.label}
                  label={card.label}
                  value={card.value}
                  detail={card.detail}
                  tone={card.tone}
                  emphasis={card.emphasis}
                  variant="light"
                />
              ))}
            </div>
          </Card>
        ))}
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.15fr_0.85fr]">
        <DashboardSnapshot
          observe={observe}
          currentAgent={currentAgent}
          latestScheduleRun={latestScheduleRun}
          latestScheduleFailure={latestScheduleFailure}
          latestWebhookRun={latestWebhookRun}
          latestWebhookFailure={latestWebhookFailure}
          toolPolicies={toolPolicies}
        />

        <div className="space-y-6">
          <BlockedRunsCard blockedRuns={blockedRuns} />

          <Card title="Schedule Alerts" subtitle="失敗中 schedule を優先表示し、resolve へつなげます" variant="light">
            <div className="space-y-3">
              {scheduleAlerts.map((alert) => (
                <div key={alert.id} className="rounded-[1.75rem] border border-amber-200 bg-amber-50/70 px-4 py-4 text-sm text-zinc-800">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="font-medium text-zinc-900">{alert.name}</p>
                      <p className="mt-1 text-xs text-zinc-500">{alert.id}</p>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      <Badge tone="pending">failures {alert.consecutiveFailureCount.toString()}</Badge>
                      {alert.stale && <Badge tone="pending">stale</Badge>}
                      <Badge tone={alert.running ? "info" : alert.enabled ? "pending" : "neutral"}>
                        {alert.running ? "running" : alert.enabled ? "enabled" : "disabled"}
                      </Badge>
                    </div>
                  </div>
                  <div className="mt-3 grid gap-2 text-xs text-zinc-600 sm:grid-cols-2">
                    <p>last success {scheduleSuccessLabel(alert)}</p>
                    <p>next {alert.nextRunAt ?? "disabled"}</p>
                  </div>
                </div>
              ))}
              {scheduleAlerts.length === 0 && <p className="text-sm text-zinc-500">no failing schedules</p>}
            </div>
            <div className="mt-4 flex items-center justify-between rounded-[1.5rem] border border-zinc-200 bg-white/70 px-4 py-3">
              <p className="text-sm text-zinc-600">{failureTarget(latestScheduleFailure, blockedRuns)}</p>
              <Link to="/schedules" className="rounded-full bg-amber-500 px-4 py-2 text-sm font-medium text-white hover:bg-amber-400">
                Resolve Alerts
              </Link>
            </div>
          </Card>

          <Card title="Quick Actions" subtitle="よく使う導線だけを短く、明るい tile で出します" variant="light">
            <div className="grid gap-3 sm:grid-cols-2">
              {[
                { to: "/chat", label: "Chat", detail: "prompt と observe を追う" },
                { to: "/runs", label: "Runs", detail: latestRun?.status ?? "run timeline" },
                { to: "/agents", label: "Agents", detail: currentAgent?.id ?? "agent control" },
                { to: "/schedules", label: "Schedules", detail: latestScheduleRun?.id ?? "timer automation" },
                { to: "/webhooks", label: "Webhooks", detail: latestWebhookRun?.id ?? "automation entry" },
                { to: "/observe", label: "Summary", detail: observe.summary?.key ?? "conversation summary" },
                { to: "/memory", label: "Memory", detail: "core / seed operations" },
              ].map(({ to, label, detail }) => (
                <Link
                  key={to}
                  to={to}
                  className="flex items-center justify-between rounded-[1.5rem] border border-zinc-200 bg-white/74 px-4 py-3 text-sm text-zinc-800 transition hover:bg-zinc-50"
                >
                  <span>
                    <span className="block font-medium text-zinc-900">{label}</span>
                    <span className="mt-1 block text-xs text-zinc-500">{detail}</span>
                  </span>
                  <ArrowRight className="h-4 w-4 text-zinc-400" />
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
            variant="dashboard"
          />
        </div>
      </div>
    </div>
  );
}
