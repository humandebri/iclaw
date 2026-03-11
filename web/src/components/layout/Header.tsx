// where: standalone/web/src/components/layout/Header.tsx
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
    <header className="flex h-16 items-center justify-between border-b border-white/10 bg-slate-950/80 px-6 backdrop-blur">
      <div>
        <h2 className="text-lg font-semibold text-white">{routeTitles[location.pathname] ?? "iclaw"}</h2>
        <p className="text-sm text-slate-400">
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
          className="inline-flex items-center gap-2 rounded-xl border border-white/10 px-3 py-2 text-sm text-slate-200 transition-colors hover:bg-white/5"
        >
          <LogOut className="h-4 w-4" />
          Sign out
        </button>
      </div>
    </header>
  );
}
