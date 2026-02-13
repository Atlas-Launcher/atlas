"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { Search, X } from "lucide-react";

import { cn } from "@/lib/utils";
import type { PersonaId, SearchIndexItem } from "@/lib/docs/types";

type DocsSearchProps = {
  items: SearchIndexItem[];
  activePersona?: PersonaId | "all";
  showResultsWhenEmpty?: boolean;
  priorityPaths?: string[];
  collapsedByDefault?: boolean;
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

export default function DocsSearch({
  items,
  activePersona = "all",
  showResultsWhenEmpty = false,
  priorityPaths = [],
  collapsedByDefault = true,
}: DocsSearchProps) {
  const containerRef = useRef<HTMLElement | null>(null);
  const [query, setQuery] = useState("");
  const [selectedPersona, setSelectedPersona] = useState<PersonaId | "all">(activePersona);
  const [isExpanded, setIsExpanded] = useState(!collapsedByDefault);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        const input = document.getElementById("docs-search-input") as HTMLInputElement | null;
        setIsExpanded(true);
        input?.focus();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    if (!collapsedByDefault) {
      return;
    }

    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node) && query.trim().length === 0) {
        setIsExpanded(false);
      }
    };

    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [collapsedByDefault, query]);

  const results = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    const priority = new Set(priorityPaths);

    return items
      .filter((item) => selectedPersona === "all" || item.persona === selectedPersona)
      .map((item) => {
        const title = item.title.toLowerCase();
        const summary = item.summary.toLowerCase();
        const headings = item.headings.join(" ").toLowerCase();
        const keywords = item.keywords.join(" ").toLowerCase();
        let score = 0;

        if (normalizedQuery.length === 0) {
          score = showResultsWhenEmpty ? 1 : 0;
        } else {
          if (title.includes(normalizedQuery)) {
            score += 6;
          }
          if (summary.includes(normalizedQuery)) {
            score += 4;
          }
          if (headings.includes(normalizedQuery)) {
            score += 3;
          }
          if (keywords.includes(normalizedQuery)) {
            score += 5;
          }
          if (item.path.toLowerCase().includes(normalizedQuery)) {
            score += 2;
          }
        }

        if (priority.has(item.path)) {
          score += normalizedQuery.length === 0 ? 3 : 2;
        }

        return { item, score };
      })
      .filter((entry) => entry.score > 0)
      .sort((a, b) => b.score - a.score || a.item.title.localeCompare(b.item.title))
      .slice(0, 10)
      .map((entry) => entry.item);
  }, [items, priorityPaths, query, selectedPersona, showResultsWhenEmpty]);

  const showExpandedPanel = isExpanded || query.trim().length > 0;

  return (
    <section
      ref={containerRef}
      className={cn(
        "relative overflow-visible rounded-lg",
        showExpandedPanel ? "z-20" : "z-0"
      )}
    >
      <label htmlFor="docs-search-input" className="sr-only">
        Search docs
      </label>
      <div className="flex items-center gap-2 rounded-md border border-[hsl(var(--border)/0.7)] bg-[var(--atlas-surface)] px-3 py-2">
        <Search className="h-4 w-4 text-[var(--atlas-ink-muted)]" aria-hidden="true" />
        <input
          id="docs-search-input"
          type="text"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onFocus={() => setIsExpanded(true)}
          placeholder="Search docs"
          className="w-full bg-transparent text-sm text-[var(--atlas-ink)] outline-none"
        />
        {query.length > 0 ? (
          <button
            type="button"
            onClick={() => setQuery("")}
            className="inline-flex h-6 w-6 items-center justify-center rounded-md text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-soft)]"
            aria-label="Clear search"
          >
            <X className="h-4 w-4" />
          </button>
        ) : null}
      </div>

      {showExpandedPanel ? (
        <div className="atlas-panel-soft absolute left-0 right-0 top-full mt-2 overflow-hidden rounded-lg border border-[hsl(var(--border)/0.85)] shadow-[0_14px_26px_-20px_rgba(0,0,0,0.55)]">
          <div className="border-b border-[hsl(var(--border)/0.8)] px-4 py-3">
            <div className="flex flex-wrap gap-2" role="tablist" aria-label="Persona filter">
              <button
                type="button"
                onClick={() => setSelectedPersona("all")}
                className={cn(
                  "rounded-md border px-2.5 py-1 text-xs font-medium transition",
                  selectedPersona === "all"
                    ? "border-[hsl(var(--primary)/0.2)] bg-[var(--atlas-inverse-bg)] text-[var(--atlas-inverse-fg)]"
                    : "border-[hsl(var(--border)/0.85)] bg-[var(--atlas-surface-strong)] text-[var(--atlas-ink-muted)] hover:text-[var(--atlas-ink)]"
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
                    "rounded-md border px-2.5 py-1 text-xs font-medium transition",
                    selectedPersona === persona
                      ? "border-[hsl(var(--primary)/0.2)] bg-[var(--atlas-inverse-bg)] text-[var(--atlas-inverse-fg)]"
                      : "border-[hsl(var(--border)/0.85)] bg-[var(--atlas-surface-strong)] text-[var(--atlas-ink-muted)] hover:text-[var(--atlas-ink)]"
                  )}
                >
                  {personaLabels[persona]}
                </button>
              ))}
            </div>
          </div>

          <div aria-live="polite">
            <div className="flex items-center justify-between px-4 py-2 text-[11px] uppercase tracking-[0.14em] text-[var(--atlas-ink-muted)]">
              <p>{query.trim().length > 0 ? "Most relevant" : "Recommended"}</p>
              {results.length > 0 ? <p>{results.length} result{results.length === 1 ? "" : "s"}</p> : null}
            </div>

            {results.length === 0 ? (
              <p className="border-t border-[hsl(var(--border)/0.8)] px-4 py-4 text-sm text-[var(--atlas-ink-muted)]">
                {query.trim().length > 0
                  ? "No docs matched. Try a command, feature name, or troubleshooting term."
                  : "Type to search docs by task, command, or error."}
              </p>
            ) : (
              <div className="max-h-[420px] overflow-y-auto">
                {results.map((result) => (
                  <Link
                    key={result.id}
                    href={result.path}
                    className="block border-t border-[hsl(var(--border)/0.8)] px-4 py-3 transition hover:bg-[var(--atlas-surface-soft)]"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <p
                        className="text-sm font-semibold text-[var(--atlas-ink)]"
                        dangerouslySetInnerHTML={{ __html: highlight(result.title, query) }}
                      />
                      <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--atlas-ink-muted)]">
                        {personaLabels[result.persona]}
                      </span>
                    </div>
                    <p
                      className="mt-1 text-xs text-[var(--atlas-ink-muted)]"
                      dangerouslySetInnerHTML={{ __html: highlight(result.summary, query) }}
                    />
                    <p className="mt-1 text-[10px] uppercase tracking-[0.14em] text-[var(--atlas-ink-muted)]">
                      {result.path}
                    </p>
                  </Link>
                ))}
              </div>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
