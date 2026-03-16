// where: iclaw/web/src/components/layout/Header.tsx
// what: Top bar that shows route title, session context, and auth actions
// why: Operators need to see at a glance which session and identity state they are working with

import { LogOut, ShieldCheck } from "lucide-react";
import { useLocation } from "react-router-dom";
import { Badge } from "@/components/ui/Card";

const routeTitles: Record<string, string> = {
  "/": "Dashboard",
  "/chat": "Chat",
  "/memory": "Memory",
  "/observe": "Observe",
};

export function Header({
  sessionId,
  onLogout,
  principal,
}: {
  sessionId: string;
  onLogout: () => Promise<void>;
  principal: string;
}) {
  const location = useLocation();
  return (
    <header className="sticky top-0 z-10 flex h-20 items-center justify-between border-b border-zinc-200/70 bg-white/55 px-6 backdrop-blur-xl">
      <div className="space-y-1">
        <p className="text-[11px] font-semibold uppercase tracking-[0.28em] text-zinc-500">Route focus</p>
        <h2 className="text-xl font-semibold tracking-[-0.02em] text-zinc-900">{routeTitles[location.pathname] ?? "iclaw"}</h2>
        <p className="text-sm text-zinc-500">
          {sessionId ? `current session: ${sessionId}` : "stateless mode"}
        </p>
      </div>

      <div className="flex items-center gap-3">
        <Badge tone="info">
          <span data-tid="principal-badge" className="inline-flex items-center gap-1.5">
            <ShieldCheck className="mr-1.5 h-3.5 w-3.5" />
            {principal ? `II ${principal}` : "Internet Identity"}
          </span>
        </Badge>
        <button
          id="logout"
          type="button"
          onClick={() => void onLogout()}
          className="inline-flex items-center gap-2 rounded-full border border-zinc-200 bg-white/80 px-4 py-2.5 text-sm text-zinc-700 transition-colors hover:bg-white"
        >
          <LogOut className="h-4 w-4" />
          Sign out
        </button>
      </div>
    </header>
  );
}
