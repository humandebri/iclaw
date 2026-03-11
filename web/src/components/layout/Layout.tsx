// where: iclaw/web/src/components/layout/Layout.tsx
// what: Shared shell for the single-canister caller console
// why: Reuse the existing dashboard cadence while keeping page routing simple for the new IC-specific UI

import { Outlet } from "react-router-dom";
import { Header } from "@/components/layout/Header";
import { Sidebar } from "@/components/layout/Sidebar";

export function Layout({
  sessionId,
  onLogout,
  principal,
}: {
  sessionId: string;
  onLogout: () => Promise<void>;
  principal: string;
}) {
  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top,_rgba(37,99,235,0.22),_transparent_45%),linear-gradient(180deg,_#020617,_#0f172a_45%,_#020617)] text-white">
      <Sidebar />
      <div className="ml-64 min-h-screen">
        <Header sessionId={sessionId} onLogout={onLogout} principal={principal} />
        <main className="min-h-[calc(100vh-4rem)] px-6 py-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
