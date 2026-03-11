// where: iclaw/web/src/components/access/AuthScreens.tsx
// what: Login and access-denied shells for the operator-only caller UI
// why: Keep App.tsx focused on state wiring instead of duplicating large auth screen markup

export function UnauthenticatedScreen({
  authError,
  onLogin,
}: {
  authError: string | null;
  onLogin: () => Promise<void>;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-slate-950 px-6 text-white">
      <div className="w-full max-w-lg rounded-[28px] border border-white/10 bg-slate-900/80 p-8 shadow-2xl shadow-slate-950/50">
        <p className="text-sm uppercase tracking-[0.24em] text-blue-200/70">Operator Access</p>
        <h1 className="mt-3 text-3xl font-semibold">iclaw</h1>
        <p className="mt-4 text-sm text-slate-400">この caller UI は運用者 principal 向けです。Internet Identity でログインし、allowlist 済み principal でのみ利用できます。</p>
        {authError && <p className="mt-4 rounded-xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm text-red-100">{authError}</p>}
        <button data-tid="login-button" type="button" onClick={() => void onLogin()} className="mt-6 w-full rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-blue-500">
          Sign in with Internet Identity
        </button>
      </div>
    </div>
  );
}

export function AccessDeniedScreen({
  principal,
  message,
  onLogout,
  onRetry,
}: {
  principal: string;
  message: string | null;
  onLogout: () => Promise<void>;
  onRetry: () => Promise<void>;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-slate-950 px-6 text-white">
      <div className="w-full max-w-xl rounded-[28px] border border-red-400/20 bg-slate-900/90 p-8 shadow-2xl shadow-slate-950/50">
        <p className="text-sm uppercase tracking-[0.24em] text-red-200/70">Access denied</p>
        <h1 className="mt-3 text-3xl font-semibold">Operator principal is required</h1>
        <p data-tid="access-denied-principal" className="mt-4 rounded-xl border border-white/10 bg-slate-950/60 px-4 py-3 text-sm text-slate-200">
          principal: {principal || "unavailable"}
        </p>
        <p className="mt-4 text-sm text-slate-400">{message ?? "この principal は canister allowlist に含まれていません。"}</p>
        <p className="mt-3 text-sm text-slate-500">allowlist の参照・更新は、すでに許可されている operator だけが Dashboard から実行できます。</p>
        <div className="mt-6 flex gap-3">
          <button type="button" onClick={() => void onLogout()} className="rounded-2xl border border-white/10 px-4 py-3 text-sm text-slate-100 hover:bg-white/5">Sign out</button>
          <button type="button" onClick={() => void onRetry()} className="rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white hover:bg-blue-500">Retry access check</button>
        </div>
      </div>
    </div>
  );
}
