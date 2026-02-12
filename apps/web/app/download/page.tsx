import type { Metadata } from "next";
import Link from "next/link";

import { resolveRelease } from "@/lib/distribution";
import type { DistributionProduct } from "@/lib/db/schema";

export const metadata: Metadata = {
  title: "Downloads | Atlas Hub",
  description: "Get the Atlas Launcher and CLI with the latest release metadata.",
};

const downloadHighlights = [
  {
    title: "Release-ready builds",
    detail: "Each release is published as immutable, versioned artifacts.",
  },
  {
    title: "Fast, incremental updates",
    detail: "Launcher updates resolve through channel and platform aware endpoints.",
  },
  {
    title: "Platform coverage",
    detail: "Windows, macOS, and Linux builds ship through one distribution API.",
  },
];

const releaseTargets = [
  { os: "windows", arch: "x64" },
  { os: "macos", arch: "arm64" },
  { os: "macos", arch: "x64" },
  { os: "linux", arch: "x64" },
  { os: "linux", arch: "arm64" },
] as const;

function formatDate(value?: string) {
  if (!value) return "-";
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(new Date(value));
}

async function resolveLatestForProduct(product: DistributionProduct) {
  for (const target of releaseTargets) {
    const release = await resolveRelease({
      product,
      os: target.os,
      arch: target.arch,
      channel: "stable",
    });
    if (release) {
      return release;
    }
  }

  return null;
}

export default async function DownloadPage() {
  const [launcherRelease, cliRelease] = await Promise.all([
    resolveLatestForProduct("launcher"),
    resolveLatestForProduct("cli"),
  ]);

  return (
    <div className="space-y-16 pt-10">
      <section className="grid gap-10 lg:grid-cols-[1.1fr_0.9fr]">
        <div className="space-y-6">
          <div className="inline-flex items-center gap-2 rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            Atlas downloads
          </div>
          <h1 className="text-4xl font-semibold leading-tight md:text-6xl">
            Pick your Atlas, install in minutes.
          </h1>
          <p className="max-w-2xl text-lg text-[var(--atlas-ink-muted)]">
            Launcher builds are ready for players and auto-updates. The CLI is optimized for pack creators and CI.
          </p>
          <div className="flex flex-wrap gap-4">
            <Link
              href="/download/app/installer/latest"
              className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
            >
              Get Launcher
            </Link>
            <Link
              href="/download/cli/installer/latest"
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
            >
              Get CLI
            </Link>
          </div>
        </div>

        <div className="space-y-4 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            Latest releases
          </p>
          <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">Launcher</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              {launcherRelease ? `v${launcherRelease.version}` : "-"} · {formatDate(launcherRelease?.published_at)}
            </p>
            <div className="mt-3 flex flex-wrap gap-2 text-xs">
              <Link
                href="/download/app"
                className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
              >
                View downloads
              </Link>
            </div>
          </div>
          <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">CLI</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              {cliRelease ? `v${cliRelease.version}` : "-"} · {formatDate(cliRelease?.published_at)}
            </p>
            <div className="mt-3 flex flex-wrap gap-2 text-xs">
              <Link
                href="/download/cli"
                className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
              >
                View downloads
              </Link>
            </div>
          </div>
        </div>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        {downloadHighlights.map((highlight) => (
          <div key={highlight.title} className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-5">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">{highlight.title}</p>
            <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{highlight.detail}</p>
          </div>
        ))}
      </section>

      <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-8 text-[var(--atlas-cream)] shadow-[0_30px_80px_rgba(15,23,42,0.25)]">
        <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
          Need something else?
        </p>
        <h2 className="mt-3 text-2xl font-semibold">Looking for product-specific installers?</h2>
        <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
          Use the dedicated launcher and CLI download pages for platform-specific artifacts.
        </p>
        <div className="mt-5 flex flex-wrap gap-3">
          <Link
            href="/download/app"
            className="rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] transition hover:border-white/60"
          >
            Launcher downloads
          </Link>
          <Link
            href="/download/cli"
            className="rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] transition hover:border-white/60"
          >
            CLI downloads
          </Link>
        </div>
      </section>
    </div>
  );
}
