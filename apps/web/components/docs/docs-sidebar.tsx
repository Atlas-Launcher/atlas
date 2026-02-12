import Link from "next/link";

import type { PersonaNavSection } from "@/lib/docs/types";

type DocsSidebarProps = {
  section: PersonaNavSection;
  activeSlug: string;
};

export default function DocsSidebar({ section, activeSlug }: DocsSidebarProps) {
  return (
    <nav aria-label={`${section.title} docs`} className="space-y-2">
      {section.items.map((item) => {
        const href = item.slug.length > 0 ? `/docs/${section.id}/${item.slug}` : `/docs/${section.id}`;
        const isActive = activeSlug === item.slug;

        return (
          <Link
            key={`${section.id}:${item.slug}`}
            href={href}
            className={
              isActive
                ? "block rounded-2xl border border-[var(--atlas-ink)]/20 bg-[var(--atlas-ink)] px-3 py-2 text-[var(--atlas-cream)]"
                : "block rounded-2xl border border-[var(--atlas-ink)]/10 bg-white px-3 py-2 text-[var(--atlas-ink-muted)] transition hover:border-[var(--atlas-ink)]/20 hover:text-[var(--atlas-ink)]"
            }
          >
            <span className="block text-sm font-semibold">{item.title}</span>
            <span
              className={
                isActive
                  ? "mt-1 block text-[10px] uppercase tracking-[0.15em] text-[var(--atlas-cream)]/70"
                  : "mt-1 block text-[10px] uppercase tracking-[0.15em] text-[var(--atlas-ink-muted)]"
              }
            >
              {item.intent}
            </span>
          </Link>
        );
      })}
    </nav>
  );
}
