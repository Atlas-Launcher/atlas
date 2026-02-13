import Image from "next/image";
import Link from "next/link";

const navLinks = [
  { href: "/", label: "Overview" },
  { href: "/docs", label: "Docs" },
  { href: "/download/app", label: "Launcher" },
  { href: "/download/cli", label: "CLI" },
  { href: "/download/runner", label: "Runner" },
];

export default function PublicNavbar() {
  return (
    <header className="relative z-10 mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-6">
      <Link href="/" className="flex items-center gap-3">
        <Image
          src="/atlas-mark.svg"
          alt="Atlas"
          width={34}
          height={34}
          className="h-[34px] w-[34px] rounded-xl"
          priority
        />
        <div>
          <p className="text-sm font-semibold uppercase tracking-[0.35em] text-[var(--atlas-ink-muted)]">
            Atlas Hub
          </p>
          <p className="text-xs text-[var(--atlas-ink-muted)]">Modpack delivery made practical</p>
        </div>
      </Link>
      <nav
        className="hidden items-center gap-6 text-sm font-medium text-[var(--atlas-ink-muted)] md:flex"
        aria-label="Primary"
      >
        {navLinks.map((link) => (
          <Link key={link.href} href={link.href} className="transition hover:text-[var(--atlas-ink)]">
            {link.label}
          </Link>
        ))}
      </nav>
      <div className="flex items-center gap-3">
        <Link
          href="/sign-up"
          className="rounded-full border border-[hsl(var(--border)/0.95)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5 hover:bg-[var(--atlas-inverse-bg)] hover:text-[var(--atlas-inverse-fg)]"
        >
          Create account
        </Link>
        <Link
          href="/dashboard"
          className="rounded-full bg-[var(--atlas-accent)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] shadow-[0_10px_30px_rgba(60,132,109,0.25)] transition hover:-translate-y-0.5"
        >
          Open creator dashboard
        </Link>
      </div>
    </header>
  );
}
