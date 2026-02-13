import type { Metadata } from "next";
import Link from "next/link";

import DocsSearch from "@/components/docs/docs-search";
import { assertDocsConfiguration, getDocsNavigation, getSearchIndex } from "@/lib/docs/content";

export const metadata: Metadata = {
  title: "Docs | Atlas Hub",
  description: "Atlas user documentation for players, creators, and server hosts.",
};

export default async function DocsLandingPage() {
  await assertDocsConfiguration();
  const [navigation, searchIndex] = await Promise.all([getDocsNavigation(), getSearchIndex()]);

  return (
    <div className="space-y-8 pb-8 pt-6">
      <section className="space-y-3">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">
          User documentation
        </p>
        <h1 className="text-4xl font-semibold leading-tight">Atlas Docs</h1>
        <p className="max-w-3xl text-sm text-[var(--atlas-ink-muted)]">
          Choose your role and follow practical guides for setup, release workflows, and troubleshooting.
        </p>
      </section>

      <DocsSearch items={searchIndex} activePersona="all" showResultsWhenEmpty />

      <section className="grid gap-4 md:grid-cols-3">
        {navigation.personas.map((section) => (
          <article key={section.id} className="atlas-panel overflow-hidden rounded-lg">
            <div className="px-4 py-4">
              <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">{section.title}</p>
              <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{section.description}</p>
            </div>

            <div className="border-t border-[hsl(var(--border)/0.8)] p-4">
              <div className="grid gap-2">
                <Link
                  href={`/docs/${section.id}/${section.startSlug}`}
                  className="rounded-md border border-[hsl(var(--border)/0.8)] px-3 py-2 text-sm font-medium text-[var(--atlas-ink)] transition hover:bg-[var(--atlas-surface-soft)]"
                >
                  Start here
                </Link>
                <Link
                  href={`/docs/${section.id}/${section.troubleshootingSlug}`}
                  className="rounded-md border border-[hsl(var(--border)/0.8)] px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-soft)] hover:text-[var(--atlas-ink)]"
                >
                  Troubleshooting
                </Link>
                <Link
                  href={`/docs/${section.id}`}
                  className="rounded-md border border-[hsl(var(--border)/0.8)] px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-soft)] hover:text-[var(--atlas-ink)]"
                >
                  Browse all {section.title.toLowerCase()} docs
                </Link>
              </div>
            </div>
          </article>
        ))}
      </section>
    </div>
  );
}
