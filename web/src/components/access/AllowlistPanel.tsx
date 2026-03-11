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
}: {
  principals: string[];
  currentPrincipal: string;
  pending: boolean;
  error: string | null;
  success: string | null;
  onRefresh: () => Promise<void>;
  onSave: (principals: string[]) => Promise<void>;
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

  return (
    <Card
      title="Operator Allowlist"
      subtitle="canister に保存される operator principal 一覧"
      actions={<Badge tone={currentPrincipalIncluded ? "good" : "warn"}>{currentPrincipalIncluded ? "caller retained" : "caller missing"}</Badge>}
    >
      <div className="space-y-4">
        <div className="flex flex-wrap gap-2">
          {principals.map((principal) => (
            <Badge key={principal} tone={principal === currentPrincipal ? "info" : "neutral"}>{principal}</Badge>
          ))}
          {principals.length === 0 && <p className="text-sm text-slate-500">allowlist is empty</p>}
        </div>

        <label className="block">
          <span className="text-sm text-slate-300">1 行 1 principal で編集</span>
          <textarea
            data-tid="allowlist-draft-input"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            rows={6}
            spellCheck={false}
            className="mt-2 w-full rounded-2xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-100 outline-none transition focus:border-blue-400/40"
          />
        </label>

        <div className="rounded-2xl border border-white/10 bg-slate-950/50 p-4 text-sm text-slate-300">
          <div className="flex items-center gap-2 text-slate-100">
            <ShieldPlus className="h-4 w-4 text-blue-300" />
            更新前チェック
          </div>
          <p className="mt-2 text-slate-400">空 allowlist は拒否されます。現在の caller を外す更新も lockout 防止のため拒否されます。</p>
        </div>

        {(localError || error) && (
          <p className="rounded-2xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100">
            {localError ?? error}
          </p>
        )}
        {success && !localError && !error && (
          <p className="rounded-2xl border border-emerald-400/20 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-100">
            {success}
          </p>
        )}

        <div className="flex flex-wrap gap-3">
          <button
            data-tid="allowlist-refresh-button"
            type="button"
            onClick={() => void onRefresh()}
            className="rounded-2xl border border-white/10 px-4 py-3 text-sm text-slate-100 hover:bg-white/5"
          >
            Reload from canister
          </button>
          <button
            data-tid="allowlist-save-button"
            type="button"
            disabled={pending}
            onClick={() => void handleSave()}
            className="rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {pending ? "Saving..." : "Save allowlist"}
          </button>
        </div>
      </div>
    </Card>
  );
}
