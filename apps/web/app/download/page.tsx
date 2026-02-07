import type { Metadata } from "next";
import Link from "next/link";

import { getLatestRelease } from "@/lib/releases";

export const metadata: Metadata = {
  title: "Downloads | Atlas Hub",
  description: "Get the Atlas Launcher and CLI with the latest releases and update feeds.",
};

const downloadHighlights = [
  {
    title: "Release-ready builds",
    detail: "Each tag publishes a signed, versioned binary you can trust.",
  },
  {
    title: "Fast, incremental updates",
    detail: "Launcher builds are ready for Tauri auto-updates when enabled.",
  },
  {
    title: "Platform coverage",
    detail: "Windows, macOS, and Linux builds ship together.",
  },
];

function formatDate(value?: string) {
  if (!value) return "—";
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(new Date(value));
}

function formatTag(tag: string | undefined, prefix: string) {
  if (!tag) return "—";
  return tag.startsWith(prefix) ? `v${tag.slice(prefix.length)}` : tag;
}

export default async function DownloadPage() {
  const [launcherRelease, cliRelease] = await Promise.all([
    getLatestRelease("launcher-v"),
    getLatestRelease("cli-v"),
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
              {formatTag(launcherRelease?.tag_name, "launcher-v")} ·{" "}
              {formatDate(launcherRelease?.published_at)}
            </p>
            <div className="mt-3 flex flex-wrap gap-2 text-xs">
              <Link
                href="/download/app"
                className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
              >
                View downloads
              </Link>
              {launcherRelease?.html_url ? (
                <a
                  href={launcherRelease.html_url}
                  className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                  rel="noreferrer"
                  target="_blank"
                >
                  Release notes
                </a>
              ) : null}
            </div>
          </div>
          <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">CLI</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              {formatTag(cliRelease?.tag_name, "cli-v")} · {formatDate(cliRelease?.published_at)}
            </p>
            <div className="mt-3 flex flex-wrap gap-2 text-xs">
              <Link
                href="/download/cli"
                className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
              >
                View downloads
              </Link>
              {cliRelease?.html_url ? (
                <a
                  href={cliRelease.html_url}
                  className="rounded-full border border-[var(--atlas-ink)]/20 px-3 py-1 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                  rel="noreferrer"
                  target="_blank"
                >
                  Release notes
                </a>
              ) : null}
            </div>
          </div>
          {!launcherRelease && !cliRelease ? (
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              Set `ATLAS_RELEASE_REPO` to show the latest releases here.
            </p>
          ) : null}
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
        <h2 className="mt-3 text-2xl font-semibold">Looking for older builds or CI artifacts?</h2>
        <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
          Visit the GitHub releases page for a complete version history and checksums.
        </p>
        <div className="mt-5 flex flex-wrap gap-3">
          <Link
            href="/download/app"
            className="rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] transition hover:border-white/60"
          >
            Launcher history
          </Link>
          <Link
            href="/download/cli"
            className="rounded-full border border-white/20 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] transition hover:border-white/60"
          >
            CLI history
          </Link>
        </div>
      </section>
    </div>
  );
}
