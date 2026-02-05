"use client";

import { useState } from "react";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import PackCard from "@/app/dashboard/components/pack-card";
import type { Pack } from "@/app/dashboard/types";

interface PacksTabProps {
  packs: Pack[];
  selectedPackId: string | null;
  onSelectPack: (packId: string) => void;
  canCreatePack: boolean;
}

export default function PacksTab({
  packs,
  selectedPackId,
  onSelectPack,
  canCreatePack,
}: PacksTabProps) {
  const [query, setQuery] = useState("");
  const filteredPacks = packs.filter((pack) => {
    const haystack = `${pack.name} ${pack.slug} ${pack.repoUrl ?? ""}`.toLowerCase();
    return haystack.includes(query.toLowerCase());
  });

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Input
          placeholder="Search packs..."
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          className="min-w-[220px] flex-1"
        />
        {canCreatePack ? (
          <Link href="/dashboard/create">
            <Button size="sm">Create Pack</Button>
          </Link>
        ) : null}
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Packs</h2>
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            {filteredPacks.length} packs
          </p>
        </div>
      </div>

      {filteredPacks.length ? (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {filteredPacks.map((pack) => (
            <PackCard
              key={pack.id}
              pack={pack}
              selected={pack.id === selectedPackId}
              onSelect={() => onSelectPack(pack.id)}
            />
          ))}
        </div>
      ) : (
        <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 text-sm text-[var(--atlas-ink-muted)]">
          No packs match that search.
        </div>
      )}
    </div>
  );
}
