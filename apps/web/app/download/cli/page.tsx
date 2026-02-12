import type { Metadata } from "next";
import Link from "next/link";

import { resolveRelease } from "@/lib/distribution";

export const metadata: Metadata = {
  title: "CLI Download | Atlas Hub",
  description: "Download the latest Atlas CLI builds for automation and CI.",
};

const platformTargets = [
  {
    id: "windows",
    label: "Windows",
    detail: "PowerShell + cmd",
    os: "windows",
    arches: ["x64"] as const,
  },
  {
    id: "macos",
    label: "macOS",
    detail: "Apple silicon + Intel",
    os: "macos",
    arches: ["arm64", "x64"] as const,
  },
  {
    id: "linux",
    label: "Linux",
    detail: "x64 + arm64",
    os: "linux",
    arches: ["x64", "arm64"] as const,
  },
] as const;

function formatDate(value?: string) {
  if (!value) return "-";
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(new Date(value));
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes)) return "-";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(value >= 10 || index === 0 ? 0 : 1)} ${units[index]}`;
}

export default async function CliDownloadPage() {
  const releases = await Promise.all(
    platformTargets.flatMap((platform) =>
      platform.arches.map(async (arch) => ({
        key: `${platform.os}-${arch}`,
        os: platform.os,
        arch,
        release: await resolveRelease({
          product: "cli",
          os: platform.os,
          arch,
          channel: "stable",
        }),
      })),
    ),
  );

  const firstRelease = releases.find((entry) => entry.release)?.release ?? null;

  return (
    <div className="space-y-12 pt-10">
      <section className="grid gap-10 lg:grid-cols-[1.1fr_0.9fr]">
        <div className="space-y-6">
          <div className="inline-flex items-center gap-2 rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            Atlas CLI
          </div>
          <h1 className="text-4xl font-semibold leading-tight md:text-6xl">Download the Atlas CLI.</h1>
          <p className="max-w-2xl text-lg text-[var(--atlas-ink-muted)]">
            Build and publish modpack blobs with a fast Rust CLI designed for automation.
          </p>
          <div className="flex flex-wrap gap-4">
            <Link
              href="/download/cli/installer/latest"
              className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
            >
              Download for my platform
            </Link>
            <Link
              href="/download"
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
            >
              All downloads
            </Link>
          </div>
        </div>

        <div className="space-y-4 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            Latest CLI release
          </p>
          <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">Atlas CLI</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              {firstRelease ? `v${firstRelease.version}` : "No release detected"} · {formatDate(firstRelease?.published_at)}
            </p>
            <p className="mt-3 text-xs text-[var(--atlas-ink-muted)]">
              {firstRelease ? "Artifacts are served via /api/v1/download/{downloadId}." : "Release artifacts will appear here once published."}
            </p>
          </div>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-3">
        {platformTargets.map((platform) => {
          const platformReleases = releases.filter((entry) => entry.os === platform.os && entry.release);
          return (
            <div key={platform.id} className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
              <p className="text-sm font-semibold text-[var(--atlas-ink)]">{platform.label}</p>
              <p className="text-xs text-[var(--atlas-ink-muted)]">{platform.detail}</p>
              <div className="mt-4 flex flex-col gap-3 text-sm">
                {platformReleases.length ? (
                  platformReleases.flatMap((entry) => {
                    const release = entry.release;
                    if (!release) return [];
                    return release.assets
                      .filter((asset) => asset.kind === "installer" || asset.kind === "binary")
                      .map((asset) => (
                        <a
                          key={`${entry.key}:${asset.download_id}`}
                          href={`/api/v1/download/${asset.download_id}`}
                          className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                          rel="noreferrer"
                          target="_blank"
                        >
                          <span className="font-medium">{asset.filename}</span>
                          <span className="text-xs text-[var(--atlas-ink-muted)]">{formatBytes(asset.size)}</span>
                        </a>
                      ));
                  })
                ) : (
                  <span className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
                    Build coming soon.
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </section>

      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            API-native downloads
          </p>
          <div className="mt-4 space-y-3 text-sm text-[var(--atlas-ink-muted)]">
            <p>CLI downloads resolve directly from distribution metadata.</p>
            <p>Stable release lookup: `/api/v1/releases/cli/latest/{{os}}/{{arch}}`</p>
            <p>Artifact redirect: `/api/v1/download/{{downloadId}}`</p>
          </div>
        </div>

        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-6 text-[var(--atlas-cream)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
            CLI usage
          </p>
          <h2 className="mt-3 text-2xl font-semibold">Automate builds in CI</h2>
          <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
            Download a binary, add it to your PATH, and point it at your pack repo.
          </p>
          <div className="mt-4 rounded-2xl border border-white/15 bg-white/10 px-4 py-3 text-xs text-[var(--atlas-cream)]/70">
            Example: `atlas build --channel dev`
          </div>
        </div>
      </section>
    </div>
  );
}
