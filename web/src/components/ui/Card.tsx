// where: iclaw/web/src/components/ui/Card.tsx
// what: Small presentation helpers shared across the IC console pages
// why: Keep the dashboard-style layout consistent without copying card classes everywhere

import type { PropsWithChildren, ReactNode } from "react";

export function Card({
  title,
  subtitle,
  actions,
  children,
}: PropsWithChildren<{ title?: string; subtitle?: string; actions?: ReactNode }>) {
  return (
    <section className="rounded-3xl border border-white/10 bg-[linear-gradient(180deg,rgba(15,23,42,0.9),rgba(15,23,42,0.72))] shadow-[0_24px_80px_rgba(2,6,23,0.42)] backdrop-blur">
      {(title || actions) && (
        <header className="flex items-start justify-between gap-4 border-b border-white/10 px-5 py-4">
          <div>
            {title && <h2 className="text-base font-semibold tracking-[0.01em] text-slate-50">{title}</h2>}
            {subtitle && <p className="mt-1 max-w-2xl text-sm leading-6 text-slate-400">{subtitle}</p>}
          </div>
          {actions}
        </header>
      )}
      <div className="px-5 py-4">{children}</div>
    </section>
  );
}

export function StatCard({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail?: string;
}) {
  return (
    <div className="rounded-3xl border border-white/10 bg-white/[0.035] p-5 shadow-[inset_0_1px_0_rgba(255,255,255,0.03)]">
      <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-slate-500">{label}</p>
      <p className="mt-3 text-2xl font-semibold text-white">{value}</p>
      {detail && <p className="mt-2 text-xs leading-5 text-slate-400">{detail}</p>}
    </div>
  );
}

export function Badge({
  tone = "neutral",
  children,
}: PropsWithChildren<{ tone?: "neutral" | "good" | "warn" | "info" }>) {
  const tones = {
    neutral: "bg-white/5 text-slate-300 border-white/10",
    good: "bg-emerald-500/10 text-emerald-300 border-emerald-400/20",
    warn: "bg-amber-500/10 text-amber-200 border-amber-400/20",
    info: "bg-blue-500/10 text-blue-200 border-blue-400/20",
  };
  return (
    <span className={`inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium ${tones[tone]}`}>
      {children}
    </span>
  );
}
