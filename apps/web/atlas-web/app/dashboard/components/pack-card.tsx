"use client";

import type { Pack } from "@/app/dashboard/types";

interface PackCardProps {
  pack: Pack;
  selected: boolean;
  onSelect: () => void;
}

export default function PackCard({ pack, selected, onSelect }: PackCardProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      className={`text-left rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-4 transition ${
        selected ? "ring-2 ring-[var(--atlas-ink)]/20" : ""
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold">{pack.name}</p>
          <p className="text-xs text-[var(--atlas-ink-muted)]">{pack.slug}</p>
        </div>
        <span className="rounded-full border border-[var(--atlas-ink)]/10 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
          Open
        </span>
      </div>
      <div className="mt-3 text-xs text-[var(--atlas-ink-muted)]">
        {pack.repoUrl ?? "No repo connected"}
      </div>
    </button>
  );
}
