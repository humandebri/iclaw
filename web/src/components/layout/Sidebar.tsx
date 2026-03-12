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
    <aside className="fixed inset-y-0 left-0 w-64 border-r border-white/10 bg-slate-950/95 px-4 py-5 backdrop-blur">
      <div className="flex items-center gap-3 border-b border-white/10 px-2 pb-5">
        <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-blue-600 font-semibold text-white shadow-lg shadow-blue-950/50">
          IC
        </div>
        <div>
          <p className="text-sm uppercase tracking-[0.24em] text-blue-200/70">Single Canister</p>
          <h1 className="text-lg font-semibold text-white">iclaw</h1>
        </div>
      </div>

      <nav className="mt-6 space-y-2">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              [
                "flex items-center gap-3 rounded-xl px-3 py-3 text-sm font-medium transition-colors",
                isActive ? "bg-blue-600 text-white" : "text-slate-300 hover:bg-white/5 hover:text-white",
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
