import Link from "next/link";

import type { PersonaNavSection } from "@/lib/docs/types";

type PersonaQuickActionsProps = {
  section: PersonaNavSection;
};

export default function PersonaQuickActions({ section }: PersonaQuickActionsProps) {
  return (
    <section className="atlas-panel rounded-lg p-4">
      <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--atlas-ink-muted)]">Common links</p>
      <div className="mt-3 grid gap-1">
        <Link
          href={`/docs/${section.id}/${section.startSlug}`}
          className="rounded-md px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-strong)] hover:text-[var(--atlas-ink)]"
        >
          Start here
        </Link>
        <Link
          href={`/docs/${section.id}/${section.troubleshootingSlug}`}
          className="rounded-md px-3 py-2 text-sm text-[var(--atlas-ink-muted)] transition hover:bg-[var(--atlas-surface-strong)] hover:text-[var(--atlas-ink)]"
        >
          Troubleshooting
        </Link>
      </div>
    </section>
  );
}
