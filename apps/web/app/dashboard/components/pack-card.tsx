"use client";

import { useMemo } from "react";

import type { Pack } from "@/app/dashboard/types";
import { toRepositoryDisplayLabel, toRepositoryWebUrl } from "@/lib/github";

interface PackCardProps {
  pack: Pack;
  selected: boolean;
  onSelect: () => void;
}

export default function PackCard({ pack, selected, onSelect }: PackCardProps) {
  const repoLabel = useMemo(() => toRepositoryDisplayLabel(pack.repoUrl), [pack.repoUrl]);
  const repoHref = useMemo(() => toRepositoryWebUrl(pack.repoUrl), [pack.repoUrl]);

  return (
    <div
      className={`text-left rounded-2xl border bg-white/80 p-4 transition hover:border-[hsl(var(--border))] hover:bg-white ${
        selected ? "border-[hsl(var(--border))] ring-2 ring-[var(--atlas-ink)]/15" : "border-[hsl(var(--border)/0.8)]"
      }`}
    >
      <button type="button" onClick={onSelect} className="w-full text-left">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="text-base font-semibold leading-tight">{pack.name}</p>
            <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{pack.slug}</p>
          </div>
          <span className="rounded-full border border-[hsl(var(--border)/0.85)] px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">
            Open
          </span>
        </div>
      </button>
      <div className="mt-3 rounded-xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] px-3 py-2 text-xs text-[var(--atlas-ink-muted)]">
        {repoHref ? (
          <a
            href={repoHref}
            target="_blank"
            rel="noreferrer"
            className="block truncate underline-offset-2 hover:underline"
            title={repoHref}
          >
            {repoLabel}
          </a>
        ) : (
          <span className="block truncate" title={repoLabel}>
            {repoLabel}
          </span>
        )}
      </div>
    </div>
  );
}
