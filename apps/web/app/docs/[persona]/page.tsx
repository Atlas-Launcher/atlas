import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import DocsSearch from "@/components/docs/docs-search";
import DocsSidebar from "@/components/docs/docs-sidebar";
import PersonaQuickActions from "@/components/docs/persona-quick-actions";
import { assertDocsConfiguration, getPersonaDocs, getPersonaSection, getSearchIndex } from "@/lib/docs/content";
import { PERSONAS, type PersonaId } from "@/lib/docs/types";

type PersonaPageProps = {
  params: Promise<{ persona: string }>;
};

export async function generateMetadata({ params }: PersonaPageProps): Promise<Metadata> {
  const { persona } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    return {};
  }

  const section = await getPersonaSection(persona as PersonaId);
  return {
    title: `${section.title} Docs | Atlas Hub`,
    description: section.description,
  };
}

export async function generateStaticParams() {
  await assertDocsConfiguration();
  return PERSONAS.map((persona) => ({ persona }));
}

export default async function PersonaDocsPage({ params }: PersonaPageProps) {
  const { persona } = await params;
  if (!PERSONAS.includes(persona as PersonaId)) {
    notFound();
  }

  const personaId = persona as PersonaId;
  const [section, docs, searchIndex] = await Promise.all([
    getPersonaSection(personaId),
    getPersonaDocs(personaId),
    getSearchIndex(),
  ]);

  return (
    <div className="grid gap-6 pb-4 pt-8 lg:grid-cols-[260px_1fr] xl:grid-cols-[280px_1fr]">
      <aside className="space-y-4 lg:sticky lg:top-6 lg:h-fit">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-4">
          <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">{section.title}</p>
          <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{section.description}</p>
        </div>
        <DocsSidebar section={section} activeSlug="" />
        <PersonaQuickActions section={section} />
      </aside>

      <section className="space-y-4">
        <DocsSearch items={searchIndex} activePersona={personaId} />

        <article className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/80 p-6">
          <h1 className="text-3xl font-semibold">{section.title} Docs</h1>
          <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">Choose a guide below or jump straight to troubleshooting.</p>

          <div className="mt-5 grid gap-3 md:grid-cols-2">
            {docs.map((doc) => (
              <Link
                key={doc.id}
                href={doc.routePath}
                className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4 transition hover:-translate-y-0.5 hover:border-[var(--atlas-ink)]/20"
              >
                <p className="text-sm font-semibold text-[var(--atlas-ink)]">{doc.title}</p>
                <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{doc.summary}</p>
                <p className="mt-2 text-[10px] uppercase tracking-[0.15em] text-[var(--atlas-ink-muted)]">{doc.intent}</p>
              </Link>
            ))}
          </div>
        </article>
      </section>
    </div>
  );
}
