import type { ReactNode } from "react";
import Link from "next/link";

const navLinks = [
  { href: "/", label: "Overview" },
  { href: "/download", label: "Downloads" },
  { href: "/download/app", label: "Launcher" },
  { href: "/download/cli", label: "CLI" },
];

export default function DownloadLayout({ children }: { children: ReactNode }) {
  return (
    <div className="atlas-grid relative min-h-screen overflow-hidden bg-[var(--atlas-cream)] text-[var(--atlas-ink)]">
      <div
        className="pointer-events-none absolute -top-40 left-[-25%] h-[520px] w-[520px] rounded-full bg-[radial-gradient(circle,_rgba(120,198,163,0.25)_0%,_rgba(120,198,163,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute -top-24 right-[-12%] h-[420px] w-[420px] rounded-full bg-[radial-gradient(circle,_rgba(169,201,231,0.32)_0%,_rgba(169,201,231,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute bottom-[-120px] left-1/2 h-[360px] w-[360px] -translate-x-1/2 rounded-full bg-[radial-gradient(circle,_rgba(244,214,160,0.35)_0%,_rgba(244,214,160,0)_70%)] blur-3xl"
        aria-hidden="true"
      />

      <header className="relative z-10 mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-6">
        <Link href="/" className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-[var(--atlas-ink)] text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)]">
            A
          </div>
          <div>
            <p className="text-sm font-semibold uppercase tracking-[0.35em] text-[var(--atlas-ink-muted)]">
              Atlas Hub
            </p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">Downloads & updates</p>
          </div>
        </Link>
        <nav className="hidden items-center gap-6 text-sm font-medium text-[var(--atlas-ink-muted)] md:flex">
          {navLinks.map((link) => (
            <Link key={link.href} href={link.href} className="transition hover:text-[var(--atlas-ink)]">
              {link.label}
            </Link>
          ))}
        </nav>
        <div className="flex items-center gap-3">
          <Link
            href="/download/app"
            className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
          >
            Get Launcher
          </Link>
          <Link
            href="/download/cli"
            className="rounded-full bg-[var(--atlas-accent)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] shadow-[0_10px_30px_rgba(60,132,109,0.25)] transition hover:-translate-y-0.5"
          >
            Get CLI
          </Link>
        </div>
      </header>

      <main className="relative z-10 mx-auto w-full max-w-6xl px-6 pb-24">{children}</main>

      <footer className="relative z-10 border-t border-[var(--atlas-ink)]/10 bg-white/60">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-6 px-6 py-8 text-xs text-[var(--atlas-ink-muted)]">
          <span>Atlas Hub downloads</span>
          <span>Fast installs. Reliable updates.</span>
        </div>
      </footer>
    </div>
  );
}
