import type { DocHeading } from "@/lib/docs/types";

type DocsTocProps = {
  headings: DocHeading[];
};

export default function DocsToc({ headings }: DocsTocProps) {
  if (headings.length === 0) {
    return null;
  }

  return (
    <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/75 p-4">
      <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">On this page</p>
      <ul className="mt-3 space-y-2">
        {headings.map((heading) => (
          <li key={heading.id}>
            <a
              href={`#${heading.id}`}
              className="text-sm text-[var(--atlas-ink-muted)] transition hover:text-[var(--atlas-ink)]"
              style={{ paddingLeft: `${Math.max(0, heading.level - 1) * 10}px` }}
            >
              {heading.text}
            </a>
          </li>
        ))}
      </ul>
    </section>
  );
}
