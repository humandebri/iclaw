// where: iclaw/web/src/pages/dashboard/BlockedRunsCard.tsx
// what: Focused dashboard card for blocked runs that require operator approval
// why: Keep Dashboard.tsx under control while preserving a clear approval-entry component

import { ArrowRight } from "lucide-react";
import { Link } from "react-router-dom";
import { Badge, Card } from "@/components/ui/Card";
import type { Run } from "@/generated/iclaw.did";

export function BlockedRunsCard({ latestBlockedRun }: { latestBlockedRun: Run | null }) {
  return (
    <Card title="Blocked Runs" subtitle="tool approval が必要な run をここから追います">
      {latestBlockedRun ? (
        <div className="space-y-3">
          <div className="rounded-3xl border border-emerald-400/20 bg-emerald-500/10 px-4 py-4 text-sm text-emerald-50">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="font-medium">{latestBlockedRun.id}</p>
                <p className="mt-1 text-xs text-emerald-100/80">
                  agent {latestBlockedRun.agent_id} · session {latestBlockedRun.session_id}
                </p>
              </div>
              <Badge tone="warn">blocked</Badge>
            </div>
            <p className="mt-3 line-clamp-2 text-sm text-emerald-50/90">{latestBlockedRun.prompt}</p>
            <p className="mt-3 text-xs text-emerald-100/80">
              pending approval {latestBlockedRun.pending_tool_calls.length} tool
              {latestBlockedRun.pending_tool_calls.length === 1 ? "" : "s"}
            </p>
          </div>
          <Link
            to="/runs"
            className="flex items-center justify-between rounded-3xl border border-white/10 bg-slate-950/60 px-4 py-3 text-sm text-slate-200 transition-colors hover:bg-white/5"
          >
            <span className="flex items-center gap-3">
              <Badge tone="info">{latestBlockedRun.pending_tool_calls[0]?.name ?? "review blocked run"}</Badge>
              Runs で承認フローを確認
            </span>
            <ArrowRight className="h-4 w-4" />
          </Link>
        </div>
      ) : (
        <p className="text-sm text-slate-500">blocked run はありません。</p>
      )}
    </Card>
  );
}
