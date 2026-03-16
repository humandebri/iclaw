// where: iclaw/web/src/components/access/AllowlistPanel.tsx
// what: Dashboard card for viewing and updating the operator allowlist
// why: Authorized operators need a first-party UI path to manage access without shell-only workflows

import { useEffect, useMemo, useState } from "react";
import { ShieldPlus } from "lucide-react";
import { Badge, Card } from "@/components/ui/Card";

function normalizeDraft(raw: string): string[] {
  const next: string[] = [];
  for (const line of raw.split("\n")) {
    const value = line.trim();
    if (value && !next.includes(value)) {
      next.push(value);
    }
  }
  return next;
}

export function AllowlistPanel({
  principals,
  currentPrincipal,
  pending,
  error,
  success,
  onRefresh,
  onSave,
  variant = "default",
}: {
  principals: string[];
  currentPrincipal: string;
  pending: boolean;
  error: string | null;
  success: string | null;
  onRefresh: () => Promise<void>;
  onSave: (principals: string[]) => Promise<void>;
  variant?: "default" | "dashboard";
}) {
  const [draft, setDraft] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(principals.join("\n"));
    setLocalError(null);
  }, [principals]);

  const draftPrincipals = useMemo(() => normalizeDraft(draft), [draft]);
  const currentPrincipalIncluded = !currentPrincipal || draftPrincipals.includes(currentPrincipal);

  const handleSave = async () => {
    if (draftPrincipals.length === 0) {
      setLocalError("少なくとも 1 つの principal を入力してください。");
      return;
    }
    if (!currentPrincipalIncluded) {
      setLocalError("現在の principal は allowlist に残す必要があります。");
      return;
    }
    setLocalError(null);
    await onSave(draftPrincipals);
  };

  const inputClassName =
    variant === "dashboard" || variant === "default"
      ? "mt-2 w-full rounded-2xl border border-zinc-200 bg-zinc-50/70 px-4 py-3 text-sm text-zinc-800 outline-none transition focus:border-sky-300"
      : "mt-2 w-full rounded-2xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-100 outline-none transition focus:border-blue-400/40";
  const noticeClassName =
    variant === "dashboard" || variant === "default"
      ? "rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4 text-sm text-zinc-600"
      : "rounded-2xl border border-white/10 bg-slate-950/50 p-4 text-sm text-slate-300";
  const noticeTitleClassName = variant === "dashboard" || variant === "default" ? "flex items-center gap-2 text-zinc-900" : "flex items-center gap-2 text-slate-100";
  const noticeIconClassName = variant === "dashboard" || variant === "default" ? "h-4 w-4 text-sky-500" : "h-4 w-4 text-blue-300";
  const noticeBodyClassName = variant === "dashboard" || variant === "default" ? "mt-2 text-zinc-500" : "mt-2 text-slate-400";
  const errorClassName =
    variant === "dashboard" || variant === "default"
      ? "rounded-2xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700"
      : "rounded-2xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100";
  const successClassName =
    variant === "dashboard" || variant === "default"
      ? "rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700"
      : "rounded-2xl border border-emerald-400/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-100";
  const refreshButtonClassName =
    variant === "dashboard" || variant === "default"
      ? "rounded-full border border-zinc-200 px-4 py-3 text-sm text-zinc-800 hover:bg-zinc-50"
      : "rounded-2xl border border-white/10 px-4 py-3 text-sm text-slate-100 hover:bg-white/5";
  const saveButtonClassName =
    variant === "dashboard" || variant === "default"
      ? "rounded-full bg-zinc-900 px-4 py-3 text-sm font-medium text-white hover:bg-zinc-800 disabled:cursor-not-allowed disabled:opacity-60"
      : "rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-60";

  return (
    <Card
      title="Operator Allowlist"
      subtitle="canister に保存される operator principal 一覧"
      variant={variant === "dashboard" || variant === "default" ? "light" : "dark"}
      actions={<Badge tone={currentPrincipalIncluded ? "success" : "pending"}>{currentPrincipalIncluded ? "caller retained" : "caller missing"}</Badge>}
    >
      <div className="space-y-4">
        <div className="flex flex-wrap gap-2">
          {principals.map((principal) => (
            <Badge key={principal} tone={principal === currentPrincipal ? "info" : "neutral"}>{principal}</Badge>
          ))}
          {principals.length === 0 && <p className="text-sm text-zinc-500">allowlist is empty</p>}
        </div>

        <label className="block">
          <span className={variant === "dashboard" || variant === "default" ? "text-sm text-zinc-600" : "text-sm text-slate-300"}>1 行 1 principal で編集</span>
          <textarea
            data-tid="allowlist-draft-input"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            rows={6}
            spellCheck={false}
            className={inputClassName}
          />
        </label>

        <div className={noticeClassName}>
          <div className={noticeTitleClassName}>
            <ShieldPlus className={noticeIconClassName} />
            更新前チェック
          </div>
          <p className={noticeBodyClassName}>空 allowlist は拒否されます。現在の caller を外す更新も lockout 防止のため拒否されます。</p>
        </div>

        {(localError || error) && (
          <p className={errorClassName}>
            {localError ?? error}
          </p>
        )}
        {success && !localError && !error && (
          <p className={successClassName}>
            {success}
          </p>
        )}

        <div className="flex flex-wrap gap-3">
          <button
            data-tid="allowlist-refresh-button"
            type="button"
            onClick={() => void onRefresh()}
            className={refreshButtonClassName}
          >
            Reload from canister
          </button>
          <button
            data-tid="allowlist-save-button"
            type="button"
            disabled={pending}
            onClick={() => void handleSave()}
            className={saveButtonClassName}
          >
            {pending ? "Saving..." : "Save allowlist"}
          </button>
        </div>
      </div>
    </Card>
  );
}
