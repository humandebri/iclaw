// where: iclaw/web/src/pages/dashboard/DashboardHero.tsx
// what: Bright summary band for the operator dashboard with primary runtime signals
// why: Operators should recognize healthy vs. attention-needed states before scanning deeper panels

import { RefreshCw } from "lucide-react";
import { Link } from "react-router-dom";
import { Badge } from "@/components/ui/Card";
import type { Agent, HealthResponse, Run } from "@/generated/iclaw.did";

type DashboardHeroProps = {
  health: HealthResponse | null;
  currentAgent: Agent | null;
  blockedRuns: Run[];
  latestScheduleFailure: Run | null;
  latestWebhookFailure: Run | null;
  runCount: number;
  blockedRunCount: number;
  onRefresh: () => Promise<void>;
};

type SummaryItem = {
  label: string;
  value: string;
  detail: string;
  tone: "neutral" | "success" | "pending" | "error" | "info";
};

function latestFailureSignal(scheduleFailure: Run | null, webhookFailure: Run | null): SummaryItem {
  if (scheduleFailure) {
    return {
      label: "Latest Failure",
      value: scheduleFailure.status,
      detail: scheduleFailure.id,
      tone: "error",
    };
  }
  if (webhookFailure) {
    return {
      label: "Latest Failure",
      value: webhookFailure.status,
      detail: webhookFailure.id,
      tone: "error",
    };
  }
  return {
    label: "Latest Failure",
    value: "quiet",
    detail: "no active failures",
    tone: "success",
  };
}

function runtimeSummary(health: HealthResponse | null): SummaryItem {
  if (!health) {
    return { label: "Runtime", value: "unknown", detail: "runtime unavailable", tone: "pending" };
  }
  return {
    label: "Runtime",
    value: health.status,
    detail: health.runtime,
    tone: health.provider_ready && health.memory_ready ? "success" : "pending",
  };
}

function blockedSummary(blockedRunCount: number, blockedRuns: Run[]): SummaryItem {
  return {
    label: "Blocked Runs",
    value: String(blockedRunCount),
    detail: blockedRunCount > 0 ? `next ${blockedRuns[0]?.id ?? "approval required"}` : "no approvals pending",
    tone: blockedRunCount > 0 ? "error" : "neutral",
  };
}

export function DashboardHero({
  health,
  currentAgent,
  blockedRuns,
  latestScheduleFailure,
  latestWebhookFailure,
  runCount,
  blockedRunCount,
  onRefresh,
}: DashboardHeroProps) {
  const summaryItems: SummaryItem[] = [
    runtimeSummary(health),
    {
      label: "Selected Agent",
      value: currentAgent?.name ?? "unassigned",
      detail: currentAgent?.status ?? "agent unavailable",
      tone: currentAgent ? "info" : "pending",
    },
    blockedSummary(blockedRunCount, blockedRuns),
    latestFailureSignal(latestScheduleFailure, latestWebhookFailure),
  ];

  return (
    <section className="overflow-hidden rounded-[2rem] border border-zinc-200/80 bg-white/75 shadow-[0_18px_44px_rgba(228,228,231,0.7)] backdrop-blur-xl">
      <div className="border-b border-zinc-200/70 bg-[linear-gradient(135deg,rgba(255,255,255,0.82),rgba(254,243,199,0.42),rgba(226,232,240,0.68))] px-5 py-5">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
          <div className="space-y-3">
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone={health?.provider_ready ? "success" : "pending"}>
                provider {health?.provider_ready ? "ready" : "degraded"}
              </Badge>
              <Badge tone={health?.memory_ready ? "success" : "pending"}>
                memory {health?.memory_ready ? "ready" : "degraded"}
              </Badge>
              <Badge tone={currentAgent?.requires_tool_approval ? "pending" : "info"}>
                approval {currentAgent?.requires_tool_approval ? "required" : "quiet"}
              </Badge>
            </div>
            <div>
              <p className="text-sm font-medium uppercase tracking-[0.24em] text-zinc-500">Operator Summary</p>
              <h2 className="mt-2 text-3xl font-semibold tracking-[-0.03em] text-zinc-950">
                canister の稼働状況と要対応シグナルを先頭で判断できる dashboard
              </h2>
              <p className="mt-2 max-w-3xl text-sm leading-7 text-zinc-600">
                正常系は静かに、blocked run と failure は見落としにくく配置して、詳細カードへ自然に降りられる構成に寄せています。
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3 xl:justify-end">
            <button
              type="button"
              onClick={() => void onRefresh()}
              className="inline-flex items-center gap-2 rounded-full border border-zinc-200 bg-white px-4 py-2.5 text-sm font-medium text-zinc-800 transition hover:bg-zinc-50"
            >
              <RefreshCw className="h-4 w-4" />
              Refresh runtime
            </button>
            <Link
              to="/runs"
              className="inline-flex items-center rounded-full bg-zinc-900 px-4 py-2.5 text-sm font-medium text-white transition hover:bg-zinc-800"
            >
              Run timeline
            </Link>
            <p className="text-xs uppercase tracking-[0.18em] text-zinc-500">total runs {runCount}</p>
          </div>
        </div>
      </div>

      <div className="grid gap-3 px-4 py-4 md:grid-cols-2 xl:grid-cols-4">
        {summaryItems.map((item) => (
          <div
            key={item.label}
            className="rounded-[1.5rem] border border-zinc-200/80 bg-white/82 px-4 py-4 shadow-sm shadow-zinc-200/50"
          >
            <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">{item.label}</p>
            <div className="mt-3 flex items-center gap-2">
              <p className="text-xl font-semibold text-zinc-950">{item.value}</p>
              <Badge tone={item.tone}>{item.tone}</Badge>
            </div>
            <p className="mt-2 text-sm text-zinc-600">{item.detail}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
