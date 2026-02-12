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
  const essentials = docs.filter(
    (doc) => doc.slug === section.startSlug || doc.slug === section.troubleshootingSlug
  );
  const secondary = docs.filter(
    (doc) => doc.slug !== section.startSlug && doc.slug !== section.troubleshootingSlug
  );
  const priorityPaths = [
    `/docs/${section.id}/${section.startSlug}`,
    `/docs/${section.id}/${section.troubleshootingSlug}`,
  ];

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
        <DocsSearch
          items={searchIndex}
          activePersona={personaId}
          showResultsWhenEmpty
          priorityPaths={priorityPaths}
        />

        <article className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/80 p-6">
          <h1 className="text-3xl font-semibold">{section.title} Docs</h1>
          <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">
            Start with essentials first, then use the reference guides as needed.
          </p>

          <div className="mt-6 grid gap-3 md:grid-cols-2">
            {essentials.map((doc) => (
              <Link
                key={doc.id}
                href={doc.routePath}
                className="rounded-2xl border border-[var(--atlas-ink)]/20 bg-gradient-to-b from-[rgba(120,198,163,0.18)] to-white p-4 transition hover:-translate-y-0.5 hover:border-[var(--atlas-ink)]/30"
              >
                <p className="text-sm font-semibold text-[var(--atlas-ink)]">{doc.title}</p>
                <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{doc.summary}</p>
                <p className="mt-2 text-[10px] uppercase tracking-[0.15em] text-[var(--atlas-ink-muted)]">{doc.intent}</p>
              </Link>
            ))}
          </div>

          {secondary.length > 0 ? (
            <div className="mt-7">
              <p className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Reference guides
              </p>
              <div className="mt-2 grid gap-2">
                {secondary.map((doc) => (
                  <Link
                    key={doc.id}
                    href={doc.routePath}
                    className="rounded-xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/45 px-4 py-3 transition hover:border-[var(--atlas-ink)]/20"
                  >
                    <p className="text-sm font-semibold text-[var(--atlas-ink)]">{doc.title}</p>
                    <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{doc.summary}</p>
                  </Link>
                ))}
              </div>
            </div>
          ) : null}
        </article>
      </section>
    </div>
  );
}
