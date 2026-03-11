// where: iclaw/web/src/pages/Dashboard.tsx
// what: Operational snapshot combining health and current-session observation
// why: Operators should see runtime readiness and session behavior on one screen before taking action

import { Link } from "react-router-dom";
import { Activity, ArrowRight, Brain, Workflow } from "lucide-react";
import { AllowlistPanel } from "@/components/access/AllowlistPanel";
import { Badge, Card, StatCard } from "@/components/ui/Card";
import type { HealthResponse } from "@/generated/iclaw.did";
import type { ObserveViewModel } from "@/types/ui";

export function Dashboard({
  health,
  observe,
  allowlist,
  loading,
  error,
  onRefresh,
  onAllowlistRefresh,
  onAllowlistSave,
}: {
  health: HealthResponse | null;
  observe: ObserveViewModel;
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
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm uppercase tracking-[0.24em] text-blue-200/60">Operator View</p>
          <h2 className="mt-2 text-3xl font-semibold text-white">Caller UX Dashboard</h2>
        </div>
        <button
          type="button"
          onClick={() => void onRefresh()}
          className="rounded-xl border border-white/10 px-4 py-2 text-sm text-slate-200 transition-colors hover:bg-white/5"
        >
          Refresh
        </button>
      </div>

      {error && (
        <div className="rounded-2xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100">
          {error}
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard label="Status" value={health?.status ?? (loading ? "loading" : "unknown")} detail={health?.runtime} />
        <StatCard label="Version" value={health?.version ?? "-"} detail={health?.provider_ready ? "provider ready" : "provider degraded"} />
        <StatCard label="History Limit" value={observe.observation ? String(observe.observation.history_limit) : "-"} detail="current session window" />
        <StatCard label="Conversation Turns" value={observe.observation ? String(observe.observation.conversation_turn_count) : "-"} detail="autosaved turns" />
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <Card
          title="Current Observation"
          subtitle="health() と agent_observe(current session) の合成ビュー"
          actions={
            <div className="flex gap-2">
              <Badge tone={health?.memory_ready ? "good" : "warn"}>memory</Badge>
              <Badge tone={health?.provider_ready ? "good" : "warn"}>provider</Badge>
            </div>
          }
        >
          <div className="grid gap-4 md:grid-cols-2">
            <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
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

            <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
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

          <div className="mt-4 flex flex-wrap gap-2">
            {(observe.observation?.auto_promoted_keys ?? []).slice(0, 6).map((key) => (
              <Badge key={key} tone="info">{key}</Badge>
            ))}
            {(observe.observation?.auto_promoted_keys ?? []).length === 0 && (
              <p className="text-sm text-slate-500">auto-promoted keys are empty</p>
            )}
          </div>
        </Card>

        <div className="space-y-6">
          <Card title="Quick Actions" subtitle="よく使う導線だけを短く出す">
            <div className="space-y-3">
              {[
                { to: "/chat", label: "Chat へ移動", icon: MessageLabel("prompt and observe") },
                { to: "/observe", label: "Summary を確認", icon: MessageLabel("conversation summary") },
                { to: "/memory", label: "Core / Seed を編集", icon: MessageLabel("memory operations") },
              ].map(({ to, label, icon }) => (
                <Link
                  key={to}
                  to={to}
                  className="flex items-center justify-between rounded-2xl border border-white/10 bg-slate-950/60 px-4 py-3 text-sm text-slate-200 transition-colors hover:bg-white/5"
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
