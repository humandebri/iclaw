// where: iclaw/web/src/pages/dashboard/BlockedRunsCard.tsx
// what: Focused dashboard card for blocked runs that require operator approval
// why: Approval-required work should read like a pending operator queue instead of a healthy state card

import { ArrowRight } from "lucide-react";
import { Link } from "react-router-dom";
import { Badge, Card } from "@/components/ui/Card";
import type { Run } from "@/generated/iclaw.did";

function blockedDetail(run: Run): string {
  return `agent ${run.agent_id} · session ${run.session_id}`;
}

export function BlockedRunsCard({ blockedRuns }: { blockedRuns: Run[] }) {
  return (
    <Card title="Blocked Runs" subtitle="tool approval が必要な run を優先順に確認します" variant="light">
      {blockedRuns.length > 0 ? (
        <div className="space-y-3">
          {blockedRuns.map((run) => (
            <Link
              key={run.id}
              to={`/runs?runId=${encodeURIComponent(run.id)}`}
              className="block rounded-[1.75rem] border border-amber-200 bg-[linear-gradient(180deg,rgba(255,251,235,0.95),rgba(255,255,255,0.92))] px-4 py-4 text-sm text-zinc-800 transition-colors hover:bg-amber-50/80"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="font-semibold text-zinc-900">{run.id}</p>
                    <Badge tone="pending">approval required</Badge>
                  </div>
                  <p className="mt-1 text-xs text-zinc-500">{blockedDetail(run)}</p>
                </div>
                <ArrowRight className="mt-1 h-4 w-4 text-zinc-400" />
              </div>
              <p className="mt-3 text-sm leading-6 text-zinc-700">{run.prompt}</p>
              <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-zinc-500">
                <Badge tone="info">{run.pending_tool_calls[0]?.name ?? "review blocked run"}</Badge>
                <span>
                  pending approval {run.pending_tool_calls.length} tool
                  {run.pending_tool_calls.length === 1 ? "" : "s"}
                </span>
                <span>created {run.created_at}</span>
              </div>
            </Link>
          ))}
        </div>
      ) : (
        <p className="text-sm text-zinc-500">blocked run はありません。</p>
      )}
    </Card>
  );
}
