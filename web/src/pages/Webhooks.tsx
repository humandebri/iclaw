// where: iclaw/web/src/pages/Webhooks.tsx
// what: Minimal webhook control-plane page for automation entry management
// why: operators need to register automation entrypoints without mixing them into run/chat screens

import { useEffect, useState } from "react";
import type { Agent, Webhook } from "@/generated/iclaw.did";
import { Badge, Card } from "@/components/ui/Card";
import type { AsyncActionState, WebhooksViewModel } from "@/types/ui";

export function WebhooksPage({
  webhooks,
  agents,
  action,
  onCreate,
  onSelectWebhook,
  onUpdate,
  onToggleEnabled,
  onRotateSecret,
}: {
  webhooks: WebhooksViewModel;
  agents: Agent[];
  action: AsyncActionState;
  onCreate: (draft: {
    id: string;
    name: string;
    agentId: string;
    sessionMode: string;
    fixedSessionId?: string;
    secret: string;
    enabled: boolean;
  }) => Promise<void>;
  onSelectWebhook: (webhookId: string) => void;
  onUpdate: (webhook: Webhook, secretOverride?: string) => Promise<void>;
  onToggleEnabled: (webhook: Webhook) => Promise<void>;
  onRotateSecret: (webhookId: string) => Promise<void>;
}) {
  const selectedWebhook = webhooks.selectedWebhook;
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [agentId, setAgentId] = useState("default");
  const [sessionMode, setSessionMode] = useState("create_new");
  const [fixedSessionId, setFixedSessionId] = useState("");
  const [secret, setSecret] = useState("");
  const [editName, setEditName] = useState("");
  const [editAgentId, setEditAgentId] = useState("default");
  const [editSessionMode, setEditSessionMode] = useState("create_new");
  const [editFixedSessionId, setEditFixedSessionId] = useState("");
  const [editSecretOverride, setEditSecretOverride] = useState("");

  useEffect(() => {
    const selected = selectedWebhook;
    if (!selected) {
      setEditName("");
      setEditAgentId("default");
      setEditSessionMode("create_new");
      setEditFixedSessionId("");
      setEditSecretOverride("");
      return;
    }
    setEditName(selected.name);
    setEditAgentId(selected.agent_id);
    setEditSessionMode(selected.session_mode);
    setEditFixedSessionId(selected.fixed_session_id[0] ?? "");
    setEditSecretOverride("");
  }, [selectedWebhook]);

  const submit = async () => {
    if (!id.trim() || !name.trim() || !secret.trim()) {
      return;
    }
    await onCreate({
      id: id.trim(),
      name: name.trim(),
      agentId,
      sessionMode,
      fixedSessionId: sessionMode === "reuse_fixed" ? fixedSessionId.trim() : undefined,
      secret: secret.trim(),
      enabled: true,
    });
    setId("");
    setName("");
    setFixedSessionId("");
    setSecret("");
  };

  const submitUpdate = async () => {
    const selected = selectedWebhook;
    if (!selected || !editName.trim()) {
      return;
    }
    await onUpdate(
      {
        ...selected,
        name: editName.trim(),
        agent_id: editAgentId,
        session_mode: editSessionMode,
        fixed_session_id:
          editSessionMode === "reuse_fixed" && editFixedSessionId.trim()
            ? [editFixedSessionId.trim()]
            : [],
      },
      editSecretOverride.trim() || undefined,
    );
    setEditSecretOverride("");
  };
  const secretNotice = extractSecretNotice(action.success);

  return (
    <div className="grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
      <Card title="Webhooks" subtitle="automation entry の一覧と状態">
        <div className="space-y-3">
          {webhooks.items.map((webhook) => (
            <button
              key={webhook.id}
              type="button"
              onClick={() => onSelectWebhook(webhook.id)}
              className={`block w-full rounded-2xl border px-4 py-3 text-left transition-colors ${
                webhooks.selectedWebhook?.id === webhook.id
                  ? "border-blue-400/40 bg-blue-500/10"
                  : "border-white/10 bg-slate-950/60 hover:bg-white/5"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-medium text-white">{webhook.name}</p>
                  <p className="mt-1 text-xs text-slate-400">{webhook.id}</p>
                </div>
                <div className="flex items-center gap-2">
                  <Badge tone="neutral">{webhook.secret}</Badge>
                  <Badge tone={webhook.enabled ? "good" : "warn"}>{webhook.enabled ? "enabled" : "disabled"}</Badge>
                </div>
              </div>
            </button>
          ))}
          {webhooks.items.length === 0 && <p className="text-sm text-slate-500">webhook はまだありません。</p>}
        </div>
      </Card>

      <div className="space-y-6">
        <Card title="Create Webhook" subtitle="prompt を外部から起動する入口を追加">
          <div className="grid gap-3 md:grid-cols-2">
            <input value={id} onChange={(event) => setId(event.target.value)} placeholder="webhook id" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white" />
            <input value={name} onChange={(event) => setName(event.target.value)} placeholder="display name" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white" />
            <select value={agentId} onChange={(event) => setAgentId(event.target.value)} className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-200">
              {agents.map((agent) => (
                <option key={agent.id} value={agent.id}>{agent.name}</option>
              ))}
            </select>
            <select value={sessionMode} onChange={(event) => setSessionMode(event.target.value)} className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-200">
              <option value="create_new">create_new</option>
              <option value="reuse_fixed">reuse_fixed</option>
            </select>
            {sessionMode === "reuse_fixed" && (
              <input value={fixedSessionId} onChange={(event) => setFixedSessionId(event.target.value)} placeholder="fixed session id" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white md:col-span-2" />
            )}
            <input value={secret} onChange={(event) => setSecret(event.target.value)} placeholder="shared secret" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white md:col-span-2" />
          </div>
          {action.error && <p className="mt-3 text-sm text-red-200">{action.error}</p>}
          {action.success && !secretNotice && <p className="mt-3 text-sm text-emerald-200">{action.success}</p>}
          {secretNotice && (
            <div className="mt-3 rounded-2xl border border-emerald-400/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-100">
              <p className="font-medium">{secretNotice.summary}</p>
              <p className="mt-2 break-all rounded-xl border border-emerald-400/20 bg-black/20 px-3 py-2 font-mono text-xs text-emerald-50">
                {secretNotice.secret}
              </p>
              <p className="mt-2 text-xs text-emerald-100/80">この secret は今しか表示されません。後から再取得はできません。</p>
            </div>
          )}
          <button type="button" onClick={() => void submit()} disabled={action.pending} className="mt-4 rounded-xl bg-blue-600 px-4 py-3 text-sm font-medium text-white disabled:bg-blue-900/60">
            {action.pending ? "Saving" : "Create webhook"}
          </button>
        </Card>

        <Card title="Webhook Detail" subtitle="query は mask 済み secret を返し、必要ならここで metadata と secret を更新">
          {!selectedWebhook && <p className="text-sm text-slate-500">webhook を選ぶと詳細が見えます。</p>}
          {selectedWebhook && (
            <div className="space-y-4 text-sm text-slate-300">
              <div className="flex flex-wrap gap-2">
                <Badge tone={selectedWebhook.enabled ? "good" : "warn"}>{selectedWebhook.session_mode}</Badge>
                <Badge tone="info">agent {selectedWebhook.agent_id}</Badge>
              </div>
              <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Current Secret</p>
                <p className="mt-3 break-all text-sm text-slate-200">{selectedWebhook.secret}</p>
                <p className="mt-2 text-xs text-slate-500">全文は create / rotate 成功時の notice で一度だけ確認できます。</p>
              </div>
              <div className="rounded-2xl border border-amber-400/20 bg-amber-500/10 p-4 text-sm text-amber-100">
                <p className="font-medium">Rotate secret は破壊的な運用操作です。</p>
                <p className="mt-2 text-xs leading-5 text-amber-100/80">
                  実行後は旧 secret が即無効になります。新 secret は success notice で一度だけ表示され、後から再取得はできません。
                </p>
              </div>
              <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Run</p>
                <p className="mt-3 text-sm text-slate-200">{webhooks.latestRuns[selectedWebhook.id]?.id ?? "none"}</p>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                  <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Last Invoked</p>
                  <p className="mt-3 text-sm text-slate-200">{selectedWebhook.last_invoked_at[0] ?? "never"}</p>
                </div>
                <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                  <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Last Secret Rotation</p>
                  <p className="mt-3 text-sm text-slate-200">{selectedWebhook.last_secret_rotated_at[0] ?? "never"}</p>
                </div>
              </div>
              <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Latest Rejection</p>
                <p className="mt-3 text-sm text-slate-200">
                  {selectedWebhook.last_rejection_reason[0]
                    ? `${selectedWebhook.last_rejection_reason[0]} @ ${selectedWebhook.last_rejection_at[0] ?? "unknown"}`
                    : "none"}
                </p>
              </div>
              <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Recent Rejections</p>
                <div className="mt-3 space-y-3">
                  {(webhooks.rejections[selectedWebhook.id] ?? []).map((rejection) => (
                    <div key={rejection.id} className="rounded-xl border border-white/10 bg-black/20 px-3 py-2">
                      <p className="text-sm text-slate-200">{rejection.reason}</p>
                      <p className="mt-1 text-xs text-slate-500">{rejection.timestamp}</p>
                    </div>
                  ))}
                  {(webhooks.rejections[selectedWebhook.id] ?? []).length === 0 && (
                    <p className="text-sm text-slate-500">rejection log はまだありません。</p>
                  )}
                </div>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <input value={editName} onChange={(event) => setEditName(event.target.value)} placeholder="display name" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white" />
                <select value={editAgentId} onChange={(event) => setEditAgentId(event.target.value)} className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-200">
                  {agents.map((agent) => (
                    <option key={agent.id} value={agent.id}>{agent.name}</option>
                  ))}
                </select>
                <select value={editSessionMode} onChange={(event) => setEditSessionMode(event.target.value)} className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-200">
                  <option value="create_new">create_new</option>
                  <option value="reuse_fixed">reuse_fixed</option>
                </select>
                {editSessionMode === "reuse_fixed" && (
                  <input value={editFixedSessionId} onChange={(event) => setEditFixedSessionId(event.target.value)} placeholder="fixed session id" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white" />
                )}
                <input value={editSecretOverride} onChange={(event) => setEditSecretOverride(event.target.value)} placeholder="new secret (optional)" className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white md:col-span-2" />
              </div>
              <button
                type="button"
                onClick={() => void submitUpdate()}
                disabled={action.pending}
                className="rounded-xl bg-slate-200 px-4 py-3 text-sm font-medium text-slate-950 disabled:bg-slate-700 disabled:text-slate-300"
              >
                {action.pending ? "Saving" : "Save detail changes"}
              </button>
              <button
                type="button"
                onClick={() => {
                  void onToggleEnabled(selectedWebhook);
                }}
                className="rounded-xl border border-white/10 px-4 py-3 text-sm text-slate-200 transition-colors hover:bg-white/5"
              >
                {selectedWebhook.enabled ? "Disable webhook" : "Enable webhook"}
              </button>
              <button
                type="button"
                onClick={() => void onRotateSecret(selectedWebhook.id)}
                className="rounded-xl border border-amber-400/20 px-4 py-3 text-sm text-amber-100 transition-colors hover:bg-amber-500/10"
              >
                Rotate secret
              </button>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}

function extractSecretNotice(success: string | null): { summary: string; secret: string } | null {
  if (!success) {
    return null;
  }
  const marker = " secret=";
  const markerIndex = success.indexOf(marker);
  if (markerIndex === -1) {
    return null;
  }
  return {
    summary: success.slice(0, markerIndex),
    secret: success.slice(markerIndex + marker.length),
  };
}
