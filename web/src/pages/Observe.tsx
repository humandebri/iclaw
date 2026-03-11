// where: iclaw/web/src/pages/Observe.tsx
// what: Read-only observation screen for the newly added observability queries
// why: Operators need one place to inspect workspace/core/session state without mixing edit controls into the view

import { Badge, Card } from "@/components/ui/Card";
import type { ObserveViewModel } from "@/types/ui";

export function ObservePage({
  sessionId,
  filter,
  setFilter,
  observe,
  onRefresh,
}: {
  sessionId: string;
  filter: string;
  setFilter: (value: string) => void;
  observe: ObserveViewModel;
  onRefresh: () => Promise<void>;
}) {
  const activeSession = filter || sessionId;

  return (
    <div className="space-y-6">
      <Card
        title="Observe"
        subtitle="agent_observe と conversation_summary_get の read-only 面"
        actions={
          <button
            type="button"
            onClick={() => void onRefresh()}
            className="rounded-xl border border-white/10 px-4 py-2 text-sm text-slate-200 transition-colors hover:bg-white/5"
          >
            Refresh
          </button>
        }
      >
        <div className="grid gap-3 md:grid-cols-[1fr_auto]">
          <input
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
            placeholder={sessionId ? `blank => current session (${sessionId})` : "optional session_id"}
            className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white placeholder:text-slate-500"
          />
          <div className="flex items-center gap-2">
            <Badge tone="info">{activeSession ? `session ${activeSession}` : "sessionless observe"}</Badge>
          </div>
        </div>
      </Card>

      <div className="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <Card title="Session Metrics">
          <dl className="grid gap-4 sm:grid-cols-2">
            <Metric label="turn count" value={String(observe.observation?.conversation_turn_count ?? 0)} />
            <Metric label="history limit" value={String(observe.observation?.history_limit ?? 0)} />
            <Metric label="tool loop" value={observe.observation?.tool_loop_enabled ? "enabled" : "disabled"} />
            <Metric label="summary" value={observe.observation?.conversation_summary_present ? "present" : "missing"} />
          </dl>
          <div className="mt-5 flex flex-wrap gap-2">
            <Badge tone={observe.observation?.enable_auto_promote ? "good" : "neutral"}>
              auto promote {observe.observation?.enable_auto_promote ? "on" : "off"}
            </Badge>
            <Badge tone={observe.observation?.enable_conversation_summary ? "good" : "neutral"}>
              summary {observe.observation?.enable_conversation_summary ? "on" : "off"}
            </Badge>
          </div>
        </Card>

        <Card title="Summary Preview">
          {observe.summary ? (
            <div className="space-y-3">
              <Badge tone="info">{observe.summary.key}</Badge>
              <p className="whitespace-pre-wrap text-sm text-slate-200">{observe.summary.content}</p>
            </div>
          ) : (
            <p className="text-sm text-slate-500">summary はまだありません。</p>
          )}
        </Card>
      </div>

      <div className="grid gap-6 xl:grid-cols-3">
        <KeyList title="Workspace Keys" keys={observe.observation?.workspace_keys ?? []} />
        <KeyList title="Core Keys" keys={observe.observation?.core_keys ?? []} />
        <KeyList title="Auto-promoted Keys" keys={observe.observation?.auto_promoted_keys ?? []} />
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
      <dt className="text-xs uppercase tracking-[0.2em] text-slate-500">{label}</dt>
      <dd className="mt-3 text-xl font-semibold text-white">{value}</dd>
    </div>
  );
}

function KeyList({ title, keys }: { title: string; keys: string[] }) {
  return (
    <Card title={title}>
      <div className="space-y-2">
        {keys.length === 0 && <p className="text-sm text-slate-500">no keys</p>}
        {keys.map((key) => (
          <div key={key} className="rounded-xl border border-white/10 bg-slate-950/60 px-3 py-2 text-sm text-slate-200">
            {key}
          </div>
        ))}
      </div>
    </Card>
  );
}
