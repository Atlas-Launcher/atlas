import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import {
  assertDocsConfiguration,
  getDocByRoute,
  getPersonaSection,
} from "@/lib/docs/content";
import { renderMarkdown } from "@/lib/docs/markdown";
import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type ReaderPersonaPageProps = {
  params: Promise<{ persona: string }>;
};

export async function generateStaticParams() {
  await assertDocsConfiguration();
  return PERSONAS.map((persona) => ({ persona }));
}

export async function generateMetadata({ params }: ReaderPersonaPageProps): Promise<Metadata> {
  const { persona } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    return {};
  }

  const doc = await getDocByRoute(persona as PersonaId, []);
  if (!doc) {
    return {};
  }

  return {
    title: `${doc.title} | Reader View`,
    description: doc.summary,
  };
}

export default async function ReaderPersonaPage({ params }: ReaderPersonaPageProps) {
  const { persona } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  const personaId = persona as PersonaId;
  const [section, doc] = await Promise.all([
    getPersonaSection(personaId),
    getDocByRoute(personaId, []),
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
      </article>
    </div>
  );
}
