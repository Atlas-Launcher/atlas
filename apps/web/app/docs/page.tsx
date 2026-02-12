import type { Metadata } from "next";
import Link from "next/link";

import DocsSearch from "@/components/docs/docs-search";
import { assertDocsConfiguration, getDocsNavigation, getSearchIndex } from "@/lib/docs/content";

export const metadata: Metadata = {
  title: "Docs | Atlas Hub",
  description: "Atlas user documentation for players, creators, and server hosts.",
};

const personaStyles: Record<string, string> = {
  player: "from-[rgba(120,198,163,0.35)] to-[rgba(120,198,163,0.05)]",
  creator: "from-[rgba(169,201,231,0.35)] to-[rgba(169,201,231,0.05)]",
  host: "from-[rgba(244,214,160,0.35)] to-[rgba(244,214,160,0.05)]",
};

export default async function DocsLandingPage() {
  await assertDocsConfiguration();
  const [navigation, searchIndex] = await Promise.all([getDocsNavigation(), getSearchIndex()]);

  return (
    <div className="space-y-8 pb-4 pt-8">
      <section className="space-y-4">
        <p className="inline-flex items-center rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
          User documentation
        </p>
        <h1 className="text-4xl font-semibold leading-tight md:text-5xl">Choose your path</h1>
        <p className="max-w-3xl text-base text-[var(--atlas-ink-muted)]">
          Start with your role and follow short, practical docs for setup, daily use, and troubleshooting.
        </p>
      </section>

      <DocsSearch items={searchIndex} activePersona="all" showResultsWhenEmpty />

      <section className="grid gap-4 md:grid-cols-3">
        {navigation.personas.map((section) => (
          <article
            key={section.id}
            className={`rounded-3xl border border-[var(--atlas-ink)]/10 bg-gradient-to-b ${personaStyles[section.id]} p-5 shadow-[0_18px_48px_rgba(15,23,42,0.08)]`}
          >
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">{section.title}</p>
            <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">{section.description}</p>

            <div className="mt-4 grid gap-2">
              <Link
                href={`/docs/${section.id}/${section.startSlug}`}
                className="rounded-2xl border border-[var(--atlas-ink)]/20 bg-white/80 px-3 py-2 text-sm font-semibold transition hover:-translate-y-0.5"
              >
                Start here
              </Link>
              <Link
                href={`/docs/${section.id}/${section.troubleshootingSlug}`}
                className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:border-[var(--atlas-ink)]/20 hover:text-[var(--atlas-ink)]"
              >
                Troubleshooting
              </Link>
              <Link
                href={`/docs/${section.id}`}
                className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:border-[var(--atlas-ink)]/20 hover:text-[var(--atlas-ink)]"
              >
                Browse all {section.title.toLowerCase()} docs
              </Link>
            </div>
          </article>
        ))}
      </section>
    </div>
  );
}
