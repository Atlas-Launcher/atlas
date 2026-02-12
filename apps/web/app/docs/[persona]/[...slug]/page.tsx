import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import DocsSearch from "@/components/docs/docs-search";
import DocsSidebar from "@/components/docs/docs-sidebar";
import DocsToc from "@/components/docs/docs-toc";
import PersonaQuickActions from "@/components/docs/persona-quick-actions";
import {
  assertDocsConfiguration,
  getAdjacentDocs,
  getAllDocParams,
  getBreadcrumbs,
  getDocByRoute,
  getPersonaSection,
  getSearchIndex,
} from "@/lib/docs/content";
import { renderMarkdown } from "@/lib/docs/markdown";
import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type DocPageProps = {
  params: Promise<{ persona: string; slug: string[] }>;
};

export async function generateStaticParams() {
  await assertDocsConfiguration();
  return getAllDocParams();
}

export async function generateMetadata({ params }: DocPageProps): Promise<Metadata> {
  const { persona, slug } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    return {};
  }

  const doc = await getDocByRoute(persona as PersonaId, slug);
  if (!doc) {
    return {};
  }

  return {
    title: `${doc.title} | Atlas Docs`,
    description: doc.summary,
  };
}

export default async function DocPage({ params }: DocPageProps) {
  const { persona, slug } = await params;

  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  const personaId = persona as PersonaId;

  const [section, doc, searchIndex, breadcrumbs, adjacent] = await Promise.all([
    getPersonaSection(personaId),
    getDocByRoute(personaId, slug),
    getSearchIndex(),
    getBreadcrumbs(personaId, slug),
    getAdjacentDocs(personaId, slug),
  ]);

  if (!doc) {
    notFound();
  }

  const html = renderMarkdown(doc.body);
  const priorityPaths = [
    doc.routePath,
    adjacent.previous?.routePath ?? "",
    adjacent.next?.routePath ?? "",
  ].filter((path) => path.length > 0);

  return (
    <div className="grid gap-6 pb-4 pt-8 lg:grid-cols-[260px_1fr] xl:grid-cols-[280px_1fr_240px]">
      <aside className="space-y-4 lg:sticky lg:top-6 lg:h-fit">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">{section.title}</p>
          <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{section.description}</p>
        </div>
        <DocsSidebar section={section} activeSlug={doc.slug} />
        <PersonaQuickActions section={section} />
      </aside>

      <section className="space-y-4">
        <DocsSearch
          items={searchIndex}
          activePersona={personaId}
          priorityPaths={priorityPaths}
        />

        <article className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/80 p-6">
          <nav aria-label="Breadcrumb" className="mb-4 flex flex-wrap items-center gap-2 text-xs text-[var(--atlas-ink-muted)]">
            {breadcrumbs.map((crumb, index) => (
              <span key={crumb.href} className="inline-flex items-center gap-2">
                <Link href={crumb.href} className="transition hover:text-[var(--atlas-ink)]">
                  {crumb.label}
                </Link>
                {index < breadcrumbs.length - 1 ? <span>/</span> : null}
              </span>
            ))}
          </nav>

          <div className="flex flex-wrap items-start justify-between gap-3">
            <h1 className="text-3xl font-semibold">{doc.title}</h1>
            <Link
              href={`/docs/read/${personaId}/${doc.slug}`}
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-[var(--atlas-cream)] px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]/35"
            >
              Reader view
            </Link>
          </div>
          <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{doc.summary}</p>

          <div className="docs-prose mt-6" dangerouslySetInnerHTML={{ __html: html }} />

          <div className="mt-8 grid gap-3 md:grid-cols-2">
            {adjacent.previous ? (
              <Link
                href={adjacent.previous.routePath}
                className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4 transition hover:border-[var(--atlas-ink)]/20"
              >
                <p className="text-[10px] uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">Previous</p>
                <p className="mt-1 text-sm font-semibold">{adjacent.previous.title}</p>
              </Link>
            ) : (
              <div />
            )}

            {adjacent.next ? (
              <Link
                href={adjacent.next.routePath}
                className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4 text-right transition hover:border-[var(--atlas-ink)]/20"
              >
                <p className="text-[10px] uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">Next</p>
                <p className="mt-1 text-sm font-semibold">{adjacent.next.title}</p>
              </Link>
            ) : null}
          </div>
        </article>
      </section>

      <aside className="hidden xl:block xl:sticky xl:top-6 xl:h-fit">
        <DocsToc headings={doc.headings} />
      </aside>
    </div>
  );
}
