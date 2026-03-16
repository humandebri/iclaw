// where: iclaw/web/src/components/ui/PageHero.tsx
// what: Shared page-level summary band for operator screens
// why: Each page should start with the same calm orientation pattern instead of bespoke headers

import type { ReactNode } from "react";

export function PageHero({
  eyebrow,
  title,
  description,
  badges,
  metrics,
  actions,
}: {
  eyebrow: string;
  title: string;
  description: string;
  badges?: ReactNode;
  metrics?: Array<{ label: string; value: string; detail?: string }>;
  actions?: ReactNode;
}) {
  return (
    <section className="relative overflow-hidden rounded-[2rem] border border-zinc-200/80 bg-white/78 p-6 shadow-lg shadow-zinc-200/60 backdrop-blur-xl">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,rgba(254,243,199,0.9),transparent_25%),radial-gradient(circle_at_82%_18%,rgba(226,232,240,0.9),transparent_26%)]" />
      <div className="relative grid gap-5 xl:grid-cols-[1.05fr_0.95fr]">
        <div className="space-y-4">
          <p className="text-[11px] font-semibold uppercase tracking-[0.28em] text-zinc-500">{eyebrow}</p>
          <div>
            <h1 className="text-3xl font-semibold tracking-[-0.03em] text-zinc-900">{title}</h1>
            <p className="mt-3 max-w-2xl text-sm leading-7 text-zinc-600">{description}</p>
          </div>
          {badges && <div className="flex flex-wrap gap-2">{badges}</div>}
          {actions && <div className="flex flex-wrap gap-3">{actions}</div>}
        </div>
        {metrics && metrics.length > 0 && (
          <div className="grid gap-3 sm:grid-cols-2">
            {metrics.map((metric) => (
              <div
                key={metric.label}
                className="rounded-[1.5rem] border border-white/70 bg-white/72 p-4 shadow-sm shadow-zinc-200/60"
              >
                <p className="text-[11px] font-semibold uppercase tracking-[0.22em] text-zinc-500">{metric.label}</p>
                <p className="mt-3 text-2xl font-semibold tracking-[-0.03em] text-zinc-900">{metric.value}</p>
                {metric.detail && <p className="mt-2 text-xs leading-5 text-zinc-500">{metric.detail}</p>}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
