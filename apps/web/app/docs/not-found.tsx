import Link from "next/link";

export default function DocsNotFound() {
  return (
    <div className="mx-auto max-w-2xl py-16">
      <div className="atlas-panel rounded-3xl p-8 text-center">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">Docs</p>
        <h1 className="mt-3 text-3xl font-semibold">Page not found</h1>
        <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">The docs page may have moved or the link is incomplete.</p>
        <div className="mt-5 flex flex-wrap justify-center gap-3">
          <Link
            href="/docs"
            className="rounded-full bg-[var(--atlas-inverse-bg)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-inverse-fg)]"
          >
            Browse docs
          </Link>
          <Link
            href="/"
            className="atlas-panel-soft rounded-full px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)]"
          >
            Back to overview
          </Link>
        </div>
      </div>
    </div>
  );
}
