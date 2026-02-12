"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Search, X } from "lucide-react";

import { cn } from "@/lib/utils";
import type { PersonaId, SearchIndexItem } from "@/lib/docs/types";

type DocsSearchProps = {
  items: SearchIndexItem[];
  activePersona?: PersonaId | "all";
};

const personaLabels: Record<PersonaId, string> = {
  player: "Player",
  creator: "Creator",
  host: "Server Host",
};

function highlight(text: string, query: string) {
  if (!query.trim()) {
    return text;
  }

  const index = text.toLowerCase().indexOf(query.toLowerCase());
  if (index === -1) {
    return text;
  }

  const before = text.slice(0, index);
  const match = text.slice(index, index + query.length);
  const after = text.slice(index + query.length);

  return `${before}<mark>${match}</mark>${after}`;
}

export default function DocsSearch({ items, activePersona = "all" }: DocsSearchProps) {
  const [query, setQuery] = useState("");
  const [selectedPersona, setSelectedPersona] = useState<PersonaId | "all">(activePersona);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        const input = document.getElementById("docs-search-input") as HTMLInputElement | null;
        input?.focus();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const results = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();

    return items
      .filter((item) => selectedPersona === "all" || item.persona === selectedPersona)
      .map((item) => {
        const searchable = [item.title, item.summary, ...item.headings, ...item.keywords].join(" ").toLowerCase();
        const score = normalizedQuery.length === 0 ? 1 : searchable.includes(normalizedQuery) ? 2 : 0;
        return { item, score };
      })
      .filter((entry) => entry.score > 0)
      .sort((a, b) => b.score - a.score || a.item.title.localeCompare(b.item.title))
      .slice(0, 12)
      .map((entry) => entry.item);
  }, [items, query, selectedPersona]);

  return (
    <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/75 p-4 shadow-[0_18px_48px_rgba(15,23,42,0.08)]">
      <div className="flex flex-col gap-3">
        <label htmlFor="docs-search-input" className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
          Search docs
        </label>

        <div className="flex items-center gap-2 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)] px-3 py-2">
          <Search className="h-4 w-4 text-[var(--atlas-ink-muted)]" aria-hidden="true" />
          <input
            id="docs-search-input"
            type="text"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search steps, commands, and troubleshooting"
            className="w-full bg-transparent text-sm text-[var(--atlas-ink)] outline-none"
          />
          {query.length > 0 ? (
            <button
              type="button"
              onClick={() => setQuery("")}
              className="inline-flex h-6 w-6 items-center justify-center rounded-full text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-ink)]/10"
              aria-label="Clear search"
            >
              <X className="h-4 w-4" />
            </button>
          ) : null}
          <kbd className="hidden rounded-md border border-[var(--atlas-ink)]/10 bg-white px-2 py-0.5 text-[10px] text-[var(--atlas-ink-muted)] md:inline-block">
            Ctrl/⌘K
          </kbd>
        </div>

        <div className="flex flex-wrap gap-2" role="tablist" aria-label="Persona filter">
          <button
            type="button"
            onClick={() => setSelectedPersona("all")}
            className={cn(
              "rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-[0.15em] transition",
              selectedPersona === "all"
                ? "border-[var(--atlas-ink)] bg-[var(--atlas-ink)] text-[var(--atlas-cream)]"
                : "border-[var(--atlas-ink)]/15 bg-white text-[var(--atlas-ink-muted)] hover:text-[var(--atlas-ink)]"
            )}
          >
            All
          </button>
          {(Object.keys(personaLabels) as PersonaId[]).map((persona) => (
            <button
              key={persona}
              type="button"
              onClick={() => setSelectedPersona(persona)}
              className={cn(
                "rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-[0.15em] transition",
                selectedPersona === persona
                  ? "border-[var(--atlas-ink)] bg-[var(--atlas-ink)] text-[var(--atlas-cream)]"
                  : "border-[var(--atlas-ink)]/15 bg-white text-[var(--atlas-ink-muted)] hover:text-[var(--atlas-ink)]"
              )}
            >
              {personaLabels[persona]}
            </button>
          ))}
        </div>

        <div className="space-y-2" aria-live="polite">
          {results.length === 0 ? (
            <p className="rounded-2xl bg-[var(--atlas-cream)]/80 px-3 py-2 text-sm text-[var(--atlas-ink-muted)]">
              No docs matched. Try a product name, command, or troubleshooting keyword.
            </p>
          ) : (
            results.map((result) => (
              <Link
                key={result.id}
                href={result.path}
                className="block rounded-2xl border border-[var(--atlas-ink)]/10 bg-white px-3 py-2 transition hover:-translate-y-0.5 hover:border-[var(--atlas-ink)]/25"
              >
                <p
                  className="text-sm font-semibold text-[var(--atlas-ink)]"
                  dangerouslySetInnerHTML={{ __html: highlight(result.title, query) }}
                />
                <p
                  className="mt-1 text-xs text-[var(--atlas-ink-muted)]"
                  dangerouslySetInnerHTML={{ __html: highlight(result.summary, query) }}
                />
                <p className="mt-1 text-[10px] uppercase tracking-[0.15em] text-[var(--atlas-ink-muted)]">
                  {personaLabels[result.persona]} · {result.path}
                </p>
              </Link>
            ))
          )}
        </div>
      </div>
    </section>
  );
}
