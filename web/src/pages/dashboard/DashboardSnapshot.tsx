// where: iclaw/web/src/pages/dashboard/DashboardSnapshot.tsx
// what: Operator-facing observation panel with summary, latest activity, and enabled signals
// why: The dashboard center should help operators decide what to inspect next without reading raw logs first

import { Brain, Workflow } from "lucide-react";
import { Badge, Card } from "@/components/ui/Card";
import type { Agent, Run, ToolPolicy } from "@/generated/iclaw.did";
import type { ObserveViewModel } from "@/types/ui";

type DashboardSnapshotProps = {
  observe: ObserveViewModel;
  currentAgent: Agent | null;
  latestScheduleRun: Run | null;
  latestScheduleFailure: Run | null;
  latestWebhookRun: Run | null;
  latestWebhookFailure: Run | null;
  toolPolicies: ToolPolicy[];
};

type LatestSnippet = {
  label: string;
  value: string;
  detail: string;
  tone: "success" | "pending" | "error" | "info";
};

function runSnippet(label: string, run: Run | null, tone: LatestSnippet["tone"]): LatestSnippet {
  if (!run) {
    return { label, value: "none", detail: "no recent activity", tone: "info" };
  }
  return {
    label,
    value: run.status,
    detail: run.id,
    tone,
  };
}

function summaryLines(
  observe: ObserveViewModel,
  currentAgent: Agent | null,
  latestScheduleFailure: Run | null,
  latestWebhookFailure: Run | null,
): string[] {
  const lines: string[] = [];
  lines.push(currentAgent ? `agent ${currentAgent.name} is ${currentAgent.status}` : "agent is not selected");
  if (latestScheduleFailure || latestWebhookFailure) {
    lines.push(`failure signal is active in ${latestScheduleFailure ? "schedule" : "webhook"} flow`);
  } else {
    lines.push("failure signal is quiet across schedule and webhook flows");
  }
  if (observe.summary) {
    lines.push(`summary memory is present at ${observe.summary.key}`);
  } else if (observe.observation?.conversation_summary_present) {
    lines.push("conversation summary is present and ready to inspect");
  } else {
    lines.push("conversation summary is still missing");
  }
  return lines;
}

export function DashboardSnapshot({
  observe,
  currentAgent,
  latestScheduleRun,
  latestScheduleFailure,
  latestWebhookRun,
  latestWebhookFailure,
  toolPolicies,
}: DashboardSnapshotProps) {
  const enabledSignals = toolPolicies
    .filter((policy) => currentAgent?.enabled_tool_names.includes(policy.tool_name))
    .slice(0, 6);
  const latestSnippets: LatestSnippet[] = [
    runSnippet("Schedule Success", latestScheduleRun, "success"),
    runSnippet("Schedule Failure", latestScheduleFailure, "error"),
    runSnippet("Webhook Success", latestWebhookRun, "success"),
    runSnippet("Webhook Failure", latestWebhookFailure, "error"),
  ];

  return (
    <Card
      title="Current Observation"
      subtitle="summary strip と latest activity を並べ、今どこを見るべきかを短く示します"
      variant="light"
      actions={
        <Badge tone={currentAgent?.requires_tool_approval ? "pending" : "success"}>
          approval {currentAgent?.requires_tool_approval ? "required" : "quiet"}
        </Badge>
      }
    >
      <div className="space-y-4">
        <div className="grid gap-3 lg:grid-cols-[1.1fr_0.9fr]">
          <div className="rounded-[1.75rem] border border-zinc-200/80 bg-[linear-gradient(135deg,rgba(255,255,255,0.96),rgba(254,243,199,0.45),rgba(226,232,240,0.7))] p-5">
            <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">Summary Strip</p>
            <div className="mt-3 space-y-2">
              {summaryLines(observe, currentAgent, latestScheduleFailure, latestWebhookFailure).map((line) => (
                <p key={line} className="text-sm leading-6 text-zinc-700">
                  {line}
                </p>
              ))}
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-[1.5rem] border border-zinc-200/80 bg-white/78 p-4">
              <div className="flex items-center gap-2 text-sm text-zinc-700">
                <Workflow className="h-4 w-4 text-sky-500" />
                Tool Loop
              </div>
              <p className="mt-3 text-2xl font-semibold text-zinc-950">
                {observe.observation?.tool_loop_enabled ? "Enabled" : "Disabled"}
              </p>
              <p className="mt-2 text-sm text-zinc-500">
                max iterations {observe.observation?.max_tool_iterations ?? "-"}
              </p>
            </div>

            <div className="rounded-[1.5rem] border border-zinc-200/80 bg-white/78 p-4">
              <div className="flex items-center gap-2 text-sm text-zinc-700">
                <Brain className="h-4 w-4 text-sky-500" />
                Summary State
              </div>
              <p className="mt-3 text-2xl font-semibold text-zinc-950">
                {observe.observation?.conversation_summary_present ? "Present" : "Missing"}
              </p>
              <p className="mt-2 text-sm text-zinc-500">
                {observe.summary?.key ?? observe.observation?.conversation_summary_key ?? "no summary key"}
              </p>
            </div>
          </div>
        </div>

        <div className="grid gap-3 md:grid-cols-2">
          {latestSnippets.map((item) => (
            <div key={item.label} className="rounded-[1.5rem] border border-zinc-200/80 bg-white/74 p-4">
              <div className="flex items-center justify-between gap-3">
                <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">{item.label}</p>
                <Badge tone={item.tone}>{item.value}</Badge>
              </div>
              <p className="mt-3 text-sm leading-6 text-zinc-700">{item.detail}</p>
            </div>
          ))}
        </div>

        <div className="rounded-[1.5rem] border border-zinc-200/80 bg-white/74 p-4">
          <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">Enabled Signals</p>
          <div className="mt-3 flex flex-wrap gap-2">
            {enabledSignals.map((policy) => (
              <Badge key={policy.tool_name} tone={policy.requires_approval || !policy.enabled ? "pending" : "info"}>
                {policy.tool_name}
              </Badge>
            ))}
            {enabledSignals.length === 0 && (observe.observation?.auto_promoted_keys ?? []).slice(0, 6).map((key) => (
              <Badge key={key} tone="info">{key}</Badge>
            ))}
            {enabledSignals.length === 0 && (observe.observation?.auto_promoted_keys ?? []).length === 0 && (
              <p className="text-sm text-zinc-500">auto-promoted keys are empty</p>
            )}
          </div>
        </div>
      </div>
    </Card>
  );
}
