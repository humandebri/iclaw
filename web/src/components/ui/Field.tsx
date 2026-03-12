// where: iclaw/web/src/components/ui/Field.tsx
// what: Small field and metric primitives shared by dense operator screens
// why: Schedule-style pages need clearer visual hierarchy without duplicating wrapper classes

import type { PropsWithChildren } from "react";

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
    info: "border-blue-400/20 bg-blue-500/10",
  };

  return (
    <div className={`rounded-2xl border p-4 ${tones[tone]}`}>
      <p className="text-[11px] uppercase tracking-[0.22em] text-slate-500">{label}</p>
      <p className="mt-2 text-xl font-semibold text-white">{value}</p>
      {detail && <p className="mt-1 text-xs text-slate-400">{detail}</p>}
    </div>
  );
}
