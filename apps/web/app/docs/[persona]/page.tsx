import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import DocsShell from "@/components/docs/docs-shell";
import DocsSearch from "@/components/docs/docs-search";
import DocsSidebar from "@/components/docs/docs-sidebar";
import PersonaContextTabs from "@/components/docs/persona-context-tabs";
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
  const sidebar = (
    <>
      <PersonaContextTabs activePersona={personaId} />
      <div>
        <DocsSearch
          items={searchIndex}
          activePersona={personaId}
          showResultsWhenEmpty
          priorityPaths={priorityPaths}
        />
      </div>
      <section className="atlas-panel-soft rounded-lg p-3">
        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">Pages</p>
        <DocsSidebar section={section} activeSlug="" />
      </section>
      <div>
        <PersonaQuickActions section={section} />
      </div>
    </>
  );

  return (
    <DocsShell sidebar={sidebar}>
      <article className="atlas-panel rounded-xl p-6 lg:p-8">
        <h1 className="text-4xl font-semibold leading-tight">{section.title} docs</h1>
        <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
          Start with the onboarding and troubleshooting guides, then use the references for daily workflows.
        </p>

        <div className="mt-8">
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">Recommended start</p>
          <div className="mt-3 grid gap-2">
            {essentials.map((doc) => (
              <Link
                key={doc.id}
                href={doc.routePath}
                className="atlas-panel-soft rounded-md px-4 py-3 transition hover:border-[hsl(var(--border)/0.95)]"
              >
                <p className="text-sm font-semibold text-[var(--atlas-ink)]">{doc.title}</p>
                <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{doc.summary}</p>
              </Link>
            ))}
          </div>
        </div>

        {secondary.length > 0 ? (
          <div className="mt-10">
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">Reference docs</p>
            <div className="atlas-panel-soft mt-3 overflow-hidden rounded-lg">
              {secondary.map((doc, index) => (
                <Link
                  key={doc.id}
                  href={doc.routePath}
                  className={`block px-4 py-3 transition hover:bg-[var(--atlas-surface-strong)] ${index > 0 ? "border-t border-[hsl(var(--border)/0.8)]" : ""}`}
                >
                  <p className="text-sm font-semibold text-[var(--atlas-ink)]">{doc.title}</p>
                  <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{doc.summary}</p>
                </Link>
              ))}
            </div>
          </div>
        ) : null}
      </article>
    </DocsShell>
  );
}
