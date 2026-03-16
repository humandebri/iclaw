// where: iclaw/web/src/components/ui/Card.tsx
// what: Small presentation helpers shared across the IC console pages
// why: Keep the dashboard-style layout consistent without copying card classes everywhere

import type { PropsWithChildren, ReactNode } from "react";

type BadgeTone = "neutral" | "success" | "pending" | "error" | "info" | "good" | "warn";
type SurfaceVariant = "dark" | "light";

export function Card({
  title,
  subtitle,
  actions,
  children,
  variant = "light",
}: PropsWithChildren<{ title?: string; subtitle?: string; actions?: ReactNode; variant?: SurfaceVariant }>) {
  const sectionClassName =
    variant === "light"
      ? "overflow-hidden rounded-[2rem] border border-zinc-200/80 bg-white/80 shadow-[0_18px_48px_rgba(228,228,231,0.75)] backdrop-blur-xl"
      : "rounded-3xl border border-white/10 bg-[linear-gradient(180deg,rgba(15,23,42,0.9),rgba(15,23,42,0.72))] shadow-[0_24px_80px_rgba(2,6,23,0.42)] backdrop-blur";
  const headerClassName =
    variant === "light"
      ? "flex items-start justify-between gap-4 border-b border-zinc-200/70 bg-white/55 px-5 py-4"
      : "flex items-start justify-between gap-4 border-b border-white/10 px-5 py-4";
  const titleClassName = variant === "light" ? "text-base font-semibold tracking-[0.01em] text-zinc-900" : "text-base font-semibold tracking-[0.01em] text-slate-50";
  const subtitleClassName = variant === "light" ? "mt-1 max-w-2xl text-sm leading-6 text-zinc-500" : "mt-1 max-w-2xl text-sm leading-6 text-slate-400";
  return (
    <section className={sectionClassName}>
      {(title || actions) && (
        <header className={headerClassName}>
          <div>
            {title && <h2 className={titleClassName}>{title}</h2>}
            {subtitle && <p className={subtitleClassName}>{subtitle}</p>}
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
  tone = "neutral",
  emphasis = "normal",
  variant = "light",
}: {
  label: string;
  value: string;
  detail?: string;
  tone?: Exclude<BadgeTone, "good" | "warn">;
  emphasis?: "normal" | "strong";
  variant?: SurfaceVariant;
}) {
  const lightTones = {
    neutral: "border-white/80 bg-white/72",
    success: "border-emerald-200 bg-emerald-50/80",
    pending: "border-amber-200 bg-amber-50/80",
    error: "border-rose-200 bg-rose-50/82",
    info: "border-slate-200 bg-slate-50/86",
  };
  const darkTones = {
    neutral: "border-white/10 bg-white/[0.035]",
    success: "border-emerald-400/20 bg-emerald-500/10",
    pending: "border-amber-400/20 bg-amber-500/10",
    error: "border-rose-400/20 bg-rose-500/10",
    info: "border-blue-400/20 bg-blue-500/10",
  };
  const tones = variant === "light" ? lightTones : darkTones;
  const labelClassName = variant === "light" ? "text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500" : "text-[11px] font-semibold uppercase tracking-[0.22em] text-slate-500";
  const valueClassName = variant === "light" ? "mt-3 text-2xl font-semibold text-zinc-900" : "mt-3 text-2xl font-semibold text-white";
  const detailClassName = variant === "light" ? "mt-2 text-xs leading-5 text-zinc-500" : "mt-2 text-xs leading-5 text-slate-400";
  return (
    <div
      className={`rounded-[1.75rem] border p-5 shadow-sm shadow-zinc-200/60 ${tones[tone]} ${
        emphasis === "strong" ? "shadow-[0_18px_30px_rgba(228,228,231,0.85)]" : ""
      }`}
    >
      <p className={labelClassName}>{label}</p>
      <p className={valueClassName}>{value}</p>
      {detail && <p className={detailClassName}>{detail}</p>}
    </div>
  );
}

export function Badge({
  tone = "neutral",
  children,
}: PropsWithChildren<{ tone?: BadgeTone }>) {
  const tones = {
    neutral: "bg-zinc-100/80 text-zinc-600 border-zinc-200",
    success: "bg-emerald-50 text-emerald-700 border-emerald-200",
    pending: "bg-amber-50 text-amber-700 border-amber-200",
    error: "bg-rose-50 text-rose-700 border-rose-200",
    info: "bg-slate-100 text-slate-700 border-slate-200",
    good: "bg-emerald-50 text-emerald-700 border-emerald-200",
    warn: "bg-amber-50 text-amber-700 border-amber-200",
  };
  return (
    <span className={`inline-flex items-center rounded-full border px-2.5 py-1 text-xs font-medium tracking-[0.02em] ${tones[tone]}`}>
      {children}
    </span>
  );
}
