import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import DocsShell from "@/components/docs/docs-shell";
import DocsSearch from "@/components/docs/docs-search";
import DocsSidebar from "@/components/docs/docs-sidebar";
import DocsToc from "@/components/docs/docs-toc";
import PersonaContextTabs from "@/components/docs/persona-context-tabs";
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
  const sidebar = (
    <>
      <PersonaContextTabs activePersona={personaId} />
      <div>
        <DocsSearch items={searchIndex} activePersona={personaId} priorityPaths={priorityPaths} />
      </div>
      <div>
        <DocsToc headings={doc.headings} />
      </div>
      <section className="atlas-panel rounded-lg p-3">
        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">Pages</p>
        <DocsSidebar section={section} activeSlug={doc.slug} />
      </section>
      <div>
        <PersonaQuickActions section={section} />
      </div>
    </>
  );

  return (
    <DocsShell sidebar={sidebar}>
      <article className="atlas-panel rounded-xl p-6 lg:p-8">
        <nav aria-label="Breadcrumb" className="flex flex-wrap items-center gap-2 text-xs text-[var(--atlas-ink-muted)]">
          {breadcrumbs.map((crumb, index) => (
            <span key={crumb.href} className="inline-flex items-center gap-2">
              <Link href={crumb.href} className="transition hover:text-[var(--atlas-ink)]">
                {crumb.label}
              </Link>
              {index < breadcrumbs.length - 1 ? <span>/</span> : null}
            </span>
          ))}
        </nav>

        <h1 className="mt-3 text-4xl font-semibold leading-tight">{doc.title}</h1>
        <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">{doc.summary}</p>

        <div className="docs-prose mt-8" dangerouslySetInnerHTML={{ __html: html }} />

        <div className="mt-10 grid gap-3 border-t border-[hsl(var(--border)/0.8)] pt-6 md:grid-cols-2">
          {adjacent.previous ? (
            <Link
              href={adjacent.previous.routePath}
              className="atlas-panel-soft rounded-md px-4 py-3 transition hover:border-[hsl(var(--border)/0.95)]"
            >
              <p className="text-[11px] uppercase tracking-[0.14em] text-[var(--atlas-ink-muted)]">Previous</p>
              <p className="mt-1 text-sm font-semibold text-[var(--atlas-ink)]">{adjacent.previous.title}</p>
            </Link>
          ) : (
            <div aria-hidden="true" />
          )}

          {adjacent.next ? (
            <Link
              href={adjacent.next.routePath}
              className="atlas-panel-soft rounded-md px-4 py-3 text-right transition hover:border-[hsl(var(--border)/0.95)]"
            >
              <p className="text-[11px] uppercase tracking-[0.14em] text-[var(--atlas-ink-muted)]">Next</p>
              <p className="mt-1 text-sm font-semibold text-[var(--atlas-ink)]">{adjacent.next.title}</p>
            </Link>
          ) : null}
        </div>
      </article>
    </DocsShell>
  );
}
