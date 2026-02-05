import type { Metadata } from "next";
import Link from "next/link";

import { getLatestRelease, getReleaseRepo, type ReleaseAsset } from "@/lib/releases";

export const metadata: Metadata = {
  title: "CLI Download | Atlas Hub",
  description: "Download the latest Atlas CLI builds for automation and CI.",
};

const platformTargets = [
  {
    id: "windows",
    label: "Windows",
    detail: "PowerShell + cmd",
    extensions: [".exe", ".msi", ".zip"],
  },
  {
    id: "macos",
    label: "macOS",
    detail: "Apple silicon + Intel",
    extensions: [".dmg", ".pkg", ".zip", ".tar.gz"],
  },
  {
    id: "linux",
    label: "Linux",
    detail: "tar.gz, deb, rpm",
    extensions: [".appimage", ".deb", ".rpm", ".tar.gz"],
  },
];

function formatDate(value?: string) {
  if (!value) return "—";
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(new Date(value));
}

function formatTag(tag: string | undefined, prefix: string) {
  if (!tag) return "No release detected";
  return tag.startsWith(prefix) ? `v${tag.slice(prefix.length)}` : tag;
}

function formatBytes(bytes: number) {
  if (!Number.isFinite(bytes)) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value.toFixed(value >= 10 || index === 0 ? 0 : 1)} ${units[index]}`;
}

function groupAssets(assets: ReleaseAsset[]) {
  const claimed = new Set<string>();
  const grouped = platformTargets.map((platform) => {
    const matches = assets.filter((asset) => {
      const name = asset.name.toLowerCase();
      return platform.extensions.some((extension) => name.endsWith(extension));
    });
    matches.forEach((asset) => claimed.add(asset.name));
    return {
      ...platform,
      assets: matches,
    };
  });

  const remaining = assets.filter((asset) => !claimed.has(asset.name));
  return { grouped, remaining };
}

export default async function CliDownloadPage() {
  const repo = getReleaseRepo();
  const release = await getLatestRelease("cli-v");
  const assets = release?.assets ?? [];
  const { grouped, remaining } = groupAssets(assets);
  const releaseTag = release?.tag_name;
  const releaseTagParam = releaseTag ? encodeURIComponent(releaseTag) : null;

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
              href="/download"
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
            >
              All downloads
            </Link>
            {release?.html_url ? (
              <a
                href={release.html_url}
                className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
                rel="noreferrer"
                target="_blank"
              >
                Release notes
              </a>
            ) : null}
          </div>
        </div>

        <div className="space-y-4 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            Latest CLI release
          </p>
          <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">{release?.name ?? "Atlas CLI"}</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              {formatTag(release?.tag_name, "cli-v")} · {formatDate(release?.published_at)}
            </p>
            <p className="mt-3 text-xs text-[var(--atlas-ink-muted)]">
              {assets.length ? `${assets.length} files available` : "Release assets will appear here once published."}
            </p>
          </div>
          {!repo ? (
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              Set `ATLAS_RELEASE_REPO` to enable automatic downloads.
            </p>
          ) : null}
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-3">
        {grouped.map((platform) => (
          <div key={platform.id} className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
            <p className="text-sm font-semibold text-[var(--atlas-ink)]">{platform.label}</p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">{platform.detail}</p>
            <div className="mt-4 flex flex-col gap-3 text-sm">
              {platform.assets.length ? (
                platform.assets.map((asset) => (
                  <a
                    key={asset.name}
                    href={
                      releaseTagParam
                        ? `/download/cli/file/${releaseTagParam}/${encodeURIComponent(asset.name)}`
                        : asset.browser_download_url
                    }
                    className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                    rel="noreferrer"
                    target="_blank"
                  >
                    <span className="font-medium">{asset.name}</span>
                    <span className="text-xs text-[var(--atlas-ink-muted)]">{formatBytes(asset.size)}</span>
                  </a>
                ))
              ) : (
                <span className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
                  Build coming soon.
                </span>
              )}
            </div>
          </div>
        ))}
      </section>

      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            Other files
          </p>
          <div className="mt-4 space-y-3">
            {remaining.length ? (
              remaining.map((asset) => (
                <a
                  key={asset.name}
                  href={
                    releaseTagParam
                      ? `/download/cli/file/${releaseTagParam}/${encodeURIComponent(asset.name)}`
                      : asset.browser_download_url
                  }
                  className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-sm text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                  rel="noreferrer"
                  target="_blank"
                >
                  <span className="font-medium">{asset.name}</span>
                  <span className="text-xs text-[var(--atlas-ink-muted)]">{formatBytes(asset.size)}</span>
                </a>
              ))
            ) : (
              <p className="text-sm text-[var(--atlas-ink-muted)]">No extra files in this release.</p>
            )}
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
