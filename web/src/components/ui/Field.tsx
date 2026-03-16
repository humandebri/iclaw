// where: iclaw/web/src/components/ui/Field.tsx
// what: Small field and metric primitives shared by dense operator screens
// why: Schedule-style pages need clearer visual hierarchy without duplicating wrapper classes

import type { PropsWithChildren } from "react";

export function controlClassName() {
  return "w-full rounded-[1.25rem] border border-zinc-200 bg-white/88 px-4 py-3 text-sm text-zinc-900 shadow-sm shadow-zinc-200/40 outline-none transition-colors placeholder:text-zinc-400 focus:border-amber-300 focus:ring-2 focus:ring-amber-100";
}

export function Field({
  label,
  hint,
  children,
}: PropsWithChildren<{ label: string; hint?: string }>) {
  return (
    <label className="block space-y-2">
      <span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-slate-500">
        {label}
      </span>
      {children}
      {hint && <p className="text-xs text-slate-500">{hint}</p>}
    </label>
  );
}

export function MetricTile({
  label,
  value,
  detail,
  tone = "neutral",
}: {
  label: string;
  value: string;
  detail?: string;
  tone?: "neutral" | "good" | "warn" | "info";
}) {
  const tones = {
    neutral: "border-white/10 bg-white/[0.03]",
    good: "border-emerald-400/20 bg-emerald-500/10",
    warn: "border-amber-400/20 bg-amber-500/10",
    info: "border-slate-200 bg-slate-100/80",
  };

  return (
    <div className={`rounded-[1.5rem] border p-4 ${tones[tone]}`}>
      <p className="text-[11px] uppercase tracking-[0.22em] text-zinc-500">{label}</p>
      <p className="mt-2 text-xl font-semibold text-zinc-900">{value}</p>
      {detail && <p className="mt-1 text-xs text-zinc-500">{detail}</p>}
    </div>
  );
}
