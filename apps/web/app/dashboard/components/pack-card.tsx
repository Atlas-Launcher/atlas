"use client";

import { useMemo } from "react";

import type { Pack } from "@/app/dashboard/types";

interface PackCardProps {
  pack: Pack;
  selected: boolean;
  onSelect: () => void;
}

export default function PackCard({ pack, selected, onSelect }: PackCardProps) {
  const repoLabel = useMemo(() => {
    if (!pack.repoUrl) {
      return "No repository linked";
    }

    try {
      const url = new URL(pack.repoUrl);
      return `${url.hostname}${url.pathname}`.replace(/\/+$/, "");
    } catch {
      return pack.repoUrl;
    }
  }, [pack.repoUrl]);

  return (
    <button
      type="button"
      onClick={onSelect}
      className={`text-left rounded-2xl border bg-white/80 p-4 transition hover:border-[var(--atlas-ink)]/25 hover:bg-white ${
        selected ? "border-[var(--atlas-ink)]/30 ring-2 ring-[var(--atlas-ink)]/15" : "border-[var(--atlas-ink)]/10"
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-base font-semibold leading-tight">{pack.name}</p>
          <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{pack.slug}</p>
        </div>
        <span className="rounded-full border border-[var(--atlas-ink)]/15 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">
          View
        </span>
      </div>
      <div className="mt-3 rounded-xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-3 py-2 text-xs text-[var(--atlas-ink-muted)]">
        <span className="block truncate" title={pack.repoUrl ?? repoLabel}>
          {repoLabel}
        </span>
      </div>
    </button>
  );
}
