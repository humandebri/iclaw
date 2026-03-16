// where: iclaw/web/src/pages/Webhooks.tsx
// what: Minimal webhook control-plane page for automation entry management
// why: operators need to register automation entrypoints without mixing them into run/chat screens

import { useEffect, useState } from "react";
import type { Agent, Webhook } from "@/generated/iclaw.did";
import { Badge, Card } from "@/components/ui/Card";
import type { AsyncActionState, WebhooksViewModel } from "@/types/ui";

const inputClassName = "rounded-xl border border-zinc-200 bg-zinc-50/80 px-4 py-3 text-sm text-zinc-900";

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
  const secretNotice = action.details.secretNotice;

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
                  ? "border-sky-300 bg-sky-50"
                  : "border-zinc-200 bg-zinc-50/70 hover:bg-white"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-medium text-zinc-900">{webhook.name}</p>
                  <p className="mt-1 text-xs text-zinc-500">{webhook.id}</p>
                </div>
                <div className="flex items-center gap-2">
                  <Badge tone="neutral">{webhook.secret}</Badge>
                  <Badge tone={webhook.enabled ? "good" : "warn"}>{webhook.enabled ? "enabled" : "disabled"}</Badge>
                </div>
              </div>
            </button>
          ))}
          {webhooks.items.length === 0 && <p className="text-sm text-zinc-500">webhook はまだありません。</p>}
        </div>
      </Card>

      <div className="space-y-6">
        <Card title="Create Webhook" subtitle="prompt を外部から起動する入口を追加">
          <div className="grid gap-3 md:grid-cols-2">
            <input value={id} onChange={(event) => setId(event.target.value)} placeholder="webhook id" className={inputClassName} />
            <input value={name} onChange={(event) => setName(event.target.value)} placeholder="display name" className={inputClassName} />
            <select value={agentId} onChange={(event) => setAgentId(event.target.value)} className={inputClassName}>
              {agents.map((agent) => (
                <option key={agent.id} value={agent.id}>{agent.name}</option>
              ))}
            </select>
            <select value={sessionMode} onChange={(event) => setSessionMode(event.target.value)} className={inputClassName}>
              <option value="create_new">create_new</option>
              <option value="reuse_fixed">reuse_fixed</option>
            </select>
            {sessionMode === "reuse_fixed" && (
              <input value={fixedSessionId} onChange={(event) => setFixedSessionId(event.target.value)} placeholder="fixed session id" className={`${inputClassName} md:col-span-2`} />
            )}
            <input value={secret} onChange={(event) => setSecret(event.target.value)} placeholder="shared secret" className={`${inputClassName} md:col-span-2`} />
          </div>
          {action.error && <p className="mt-3 text-sm text-rose-700">{action.error}</p>}
          {action.successMessage && !secretNotice && <p className="mt-3 text-sm text-emerald-700">{action.successMessage}</p>}
          {secretNotice && (
            <div className="mt-3 rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700">
              <p className="font-medium">{secretNotice.summary}</p>
              <p className="mt-2 break-all rounded-xl border border-emerald-200 bg-white px-3 py-2 font-mono text-xs text-emerald-700">
                {secretNotice.secret}
              </p>
              <p className="mt-2 text-xs text-emerald-600">この secret は今しか表示されません。後から再取得はできません。</p>
            </div>
          )}
          <button type="button" onClick={() => void submit()} disabled={action.pending} className="mt-4 rounded-xl bg-blue-600 px-4 py-3 text-sm font-medium text-white disabled:bg-blue-900/60">
            {action.pending ? "Saving" : "Create webhook"}
          </button>
        </Card>

        <Card title="Webhook Detail" subtitle="query は mask 済み secret を返し、必要ならここで metadata と secret を更新">
          {!selectedWebhook && <p className="text-sm text-zinc-500">webhook を選ぶと詳細が見えます。</p>}
          {selectedWebhook && (
            <div className="space-y-4 text-sm text-zinc-600">
              <div className="flex flex-wrap gap-2">
                <Badge tone={selectedWebhook.enabled ? "good" : "warn"}>{selectedWebhook.session_mode}</Badge>
                <Badge tone="info">agent {selectedWebhook.agent_id}</Badge>
              </div>
              <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Current Secret</p>
                <p className="mt-3 break-all text-sm text-zinc-700">{selectedWebhook.secret}</p>
                <p className="mt-2 text-xs text-zinc-500">全文は create / rotate 成功時の notice で一度だけ確認できます。</p>
              </div>
              <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4 text-sm text-amber-700">
                <p className="font-medium">Rotate secret は破壊的な運用操作です。</p>
                <p className="mt-2 text-xs leading-5 text-amber-600">
                  実行後は旧 secret が即無効になります。新 secret は success notice で一度だけ表示され、後から再取得はできません。
                </p>
              </div>
              <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Latest Run</p>
                <p className="mt-3 text-sm text-zinc-700">{webhooks.latestRuns[selectedWebhook.id]?.id ?? "none"}</p>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                  <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Last Invoked</p>
                  <p className="mt-3 text-sm text-zinc-700">{selectedWebhook.last_invoked_at[0] ?? "never"}</p>
                </div>
                <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                  <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Last Secret Rotation</p>
                  <p className="mt-3 text-sm text-zinc-700">{selectedWebhook.last_secret_rotated_at[0] ?? "never"}</p>
                </div>
              </div>
              <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Latest Rejection</p>
                <p className="mt-3 text-sm text-zinc-700">
                  {selectedWebhook.last_rejection_reason[0]
                    ? `${selectedWebhook.last_rejection_reason[0]} @ ${selectedWebhook.last_rejection_at[0] ?? "unknown"}`
                    : "none"}
                </p>
              </div>
              <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Recent Rejections</p>
                <div className="mt-3 space-y-3">
                  {(webhooks.rejections[selectedWebhook.id] ?? []).map((rejection) => (
                    <div key={rejection.id} className="rounded-xl border border-zinc-200 bg-white/70 px-3 py-2">
                      <p className="text-sm text-zinc-700">{rejection.reason}</p>
                      <p className="mt-1 text-xs text-zinc-500">{rejection.timestamp}</p>
                    </div>
                  ))}
                  {(webhooks.rejections[selectedWebhook.id] ?? []).length === 0 && (
                    <p className="text-sm text-zinc-500">rejection log はまだありません。</p>
                  )}
                </div>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <input value={editName} onChange={(event) => setEditName(event.target.value)} placeholder="display name" className={inputClassName} />
                <select value={editAgentId} onChange={(event) => setEditAgentId(event.target.value)} className={inputClassName}>
                  {agents.map((agent) => (
                    <option key={agent.id} value={agent.id}>{agent.name}</option>
                  ))}
                </select>
                <select value={editSessionMode} onChange={(event) => setEditSessionMode(event.target.value)} className={inputClassName}>
                  <option value="create_new">create_new</option>
                  <option value="reuse_fixed">reuse_fixed</option>
                </select>
                {editSessionMode === "reuse_fixed" && (
                  <input value={editFixedSessionId} onChange={(event) => setEditFixedSessionId(event.target.value)} placeholder="fixed session id" className={inputClassName} />
                )}
                <input value={editSecretOverride} onChange={(event) => setEditSecretOverride(event.target.value)} placeholder="new secret (optional)" className={`${inputClassName} md:col-span-2`} />
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
                className="rounded-xl border border-zinc-200 px-4 py-3 text-sm text-zinc-800 transition-colors hover:bg-zinc-50"
              >
                {selectedWebhook.enabled ? "Disable webhook" : "Enable webhook"}
              </button>
              <button
                type="button"
                onClick={() => void onRotateSecret(selectedWebhook.id)}
                className="rounded-xl border border-amber-200 px-4 py-3 text-sm text-amber-700 transition-colors hover:bg-amber-50"
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
