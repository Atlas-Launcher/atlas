import Link from "next/link";

import type { PersonaNavSection } from "@/lib/docs/types";

type PersonaQuickActionsProps = {
  section: PersonaNavSection;
};

export default function PersonaQuickActions({ section }: PersonaQuickActionsProps) {
  return (
    <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-5 text-[var(--atlas-cream)]">
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-accent-light)]">Quick actions</p>
      <p className="mt-2 text-sm text-[var(--atlas-cream)]/75">Jump to the most common next step for this persona.</p>
      <div className="mt-4 grid gap-2">
        <Link
          href={`/docs/${section.id}/${section.startSlug}`}
          className="rounded-2xl border border-white/15 bg-white/10 px-3 py-2 text-sm font-semibold transition hover:bg-white/20"
        >
          Start here
        </Link>
        <Link
          href={`/docs/${section.id}/${section.troubleshootingSlug}`}
          className="rounded-2xl border border-white/15 bg-white/10 px-3 py-2 text-sm font-semibold transition hover:bg-white/20"
        >
          Troubleshooting
        </Link>
      </div>
    </section>
  );
}
