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
    <div className="min-h-screen bg-[radial-gradient(circle_at_top_left,_rgba(254,243,199,0.8),_transparent_22%),radial-gradient(circle_at_80%_12%,_rgba(226,232,240,0.9),_transparent_28%),linear-gradient(180deg,_#fafaf9,_#f4f4f5_52%,_#fafaf9)] text-zinc-900">
      <Sidebar />
      <div className="relative ml-64 min-h-screen">
        <Header sessionId={sessionId} onLogout={onLogout} principal={principal} />
        <main className="min-h-[calc(100vh-4rem)] px-6 py-6 xl:px-8">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
