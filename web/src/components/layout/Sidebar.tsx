// where: iclaw/web/src/components/layout/Sidebar.tsx
// what: Primary navigation for the single-canister caller console
// why: The UI intentionally mirrors the existing iclaw dashboard information architecture

import { NavLink } from "react-router-dom";
import { BellRing, Brain, CalendarClock, Eye, LayoutDashboard, ListTree, MessageSquare, SlidersHorizontal } from "lucide-react";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/chat", label: "Chat", icon: MessageSquare },
  { to: "/runs", label: "Runs", icon: ListTree },
  { to: "/agents", label: "Agents", icon: SlidersHorizontal },
  { to: "/schedules", label: "Schedules", icon: CalendarClock },
  { to: "/webhooks", label: "Webhooks", icon: BellRing },
  { to: "/memory", label: "Memory", icon: Brain },
  { to: "/observe", label: "Observe", icon: Eye },
];

export function Sidebar() {
  return (
    <aside className="fixed inset-y-4 left-4 z-20 w-64 rounded-[2rem] border border-zinc-200/80 bg-white/72 px-4 py-5 shadow-lg shadow-zinc-200/60 backdrop-blur-xl">
      <div className="flex items-center gap-3 border-b border-zinc-200/80 px-2 pb-5">
        <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-zinc-900 font-semibold text-white shadow-sm shadow-zinc-300">
          IC
        </div>
        <div>
          <p className="text-sm uppercase tracking-[0.24em] text-zinc-500">Single Canister</p>
          <h1 className="text-lg font-semibold text-zinc-900">iclaw</h1>
          <p className="mt-1 text-xs text-zinc-500">operator console</p>
        </div>
      </div>

      <div className="mt-5 rounded-[1.5rem] border border-zinc-200 bg-zinc-50/90 px-4 py-3">
        <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">Control room</p>
        <p className="mt-2 text-sm leading-6 text-zinc-600">health, runs, schedules, webhooks をひとつの導線で横断します。</p>
      </div>

      <nav className="mt-6 space-y-2">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              [
                "flex items-center gap-3 rounded-2xl px-3 py-3 text-sm font-medium transition-all",
                isActive
                  ? "border border-zinc-300 bg-white text-zinc-900 shadow-sm shadow-zinc-200/80"
                  : "text-zinc-600 hover:bg-white/70 hover:text-zinc-900",
              ].join(" ")
            }
          >
            <Icon className="h-5 w-5" />
            {label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
