import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import {
  assertDocsConfiguration,
  getAdjacentDocs,
  getAllDocParams,
  getDocByRoute,
  getPersonaSection,
} from "@/lib/docs/content";
import { renderMarkdown } from "@/lib/docs/markdown";
import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type ReaderPageProps = {
  params: Promise<{ persona: string; slug: string[] }>;
};

function readerRoute(persona: PersonaId, slug: string) {
  return slug.length > 0 ? `/docs/read/${persona}/${slug}` : `/docs/read/${persona}`;
}

export async function generateStaticParams() {
  await assertDocsConfiguration();
  return getAllDocParams();
}

export async function generateMetadata({ params }: ReaderPageProps): Promise<Metadata> {
  const { persona, slug } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    return {};
  }

  const doc = await getDocByRoute(persona as PersonaId, slug);
  if (!doc) {
    return {};
  }

  return {
    title: `${doc.title} | Reader View`,
    description: doc.summary,
  };
}

export default async function ReaderPage({ params }: ReaderPageProps) {
  const { persona, slug } = await params;

  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  const personaId = persona as PersonaId;
  const [section, doc, adjacent] = await Promise.all([
    getPersonaSection(personaId),
    getDocByRoute(personaId, slug),
    getAdjacentDocs(personaId, slug),
  ]);

  if (!doc) {
    notFound();
  }

  const html = renderMarkdown(doc.body);

  return (
    <div className="mx-auto max-w-3xl pb-10 pt-10">
      <article className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/90 p-8 shadow-[0_22px_60px_rgba(15,23,42,0.08)]">
        <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
          <div>
            <p className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
              {section.title} · Reader view
            </p>
            <h1 className="mt-2 text-4xl font-semibold leading-tight">{doc.title}</h1>
            <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{doc.summary}</p>
          </div>
          <Link
            href={doc.routePath}
            className="rounded-full border border-[var(--atlas-ink)]/20 bg-[var(--atlas-cream)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]/35"
          >
            Standard view
          </Link>
        </div>

        <div className="docs-prose" dangerouslySetInnerHTML={{ __html: html }} />

        <div className="mt-10 grid gap-3 md:grid-cols-2">
          {adjacent.previous ? (
            <Link
              href={readerRoute(personaId, adjacent.previous.slug)}
              className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/60 p-4 transition hover:border-[var(--atlas-ink)]/20"
            >
              <p className="text-[10px] uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">Previous</p>
              <p className="mt-1 text-sm font-semibold">{adjacent.previous.title}</p>
            </Link>
          ) : (
            <div />
          )}

          {adjacent.next ? (
            <Link
              href={readerRoute(personaId, adjacent.next.slug)}
              className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/60 p-4 text-right transition hover:border-[var(--atlas-ink)]/20"
            >
              <p className="text-[10px] uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">Next</p>
              <p className="mt-1 text-sm font-semibold">{adjacent.next.title}</p>
            </Link>
          ) : null}
        </div>
      </article>
    </div>
  );
}
