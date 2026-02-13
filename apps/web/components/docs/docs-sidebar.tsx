import Link from "next/link";

import type { PersonaNavSection } from "@/lib/docs/types";

type DocsSidebarProps = {
  section: PersonaNavSection;
  activeSlug: string;
};

export default function DocsSidebar({ section, activeSlug }: DocsSidebarProps) {
  return (
    <nav aria-label={`${section.title} docs`} className="space-y-1">
      {section.items.map((item) => {
        const href = item.slug.length > 0 ? `/docs/${section.id}/${item.slug}` : `/docs/${section.id}`;
        const isActive = activeSlug === item.slug;

        return (
          <Link
            key={`${section.id}:${item.slug}`}
            href={href}
            className={
              isActive
                ? "block rounded-md border-l-2 border-[var(--atlas-accent)] bg-[var(--atlas-accent)]/12 px-3 py-2 text-[var(--atlas-ink)]"
                : "block rounded-md border-l-2 border-transparent px-3 py-2 text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-strong)] hover:text-[var(--atlas-ink)]"
            }
          >
            <span className="block text-sm font-medium">{item.title}</span>
          </Link>
        );
      })}
    </nav>
  );
}
