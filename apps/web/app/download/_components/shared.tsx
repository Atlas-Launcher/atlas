import type { ReactNode } from "react";
import Link from "next/link";

import type { DistributionReleaseResponse } from "@/lib/distribution";

export type ProductPlatformTarget = {
  id: string;
  label: string;
  detail: string;
  os: "windows" | "macos" | "linux";
  arches: readonly ("x64" | "arm64")[];
};

export type ProductPlatformAsset = {
  id: string;
  label: string;
  detail: string;
  downloadId: string;
  size: number;
};

export type ProductPlatformGroup = {
  id: string;
  label: string;
  detail: string;
  assets: ProductPlatformAsset[];
};

export function formatDate(value?: string) {
  if (!value) return "-";
  return new Intl.DateTimeFormat("en-US", { dateStyle: "medium" }).format(new Date(value));
}

export function formatBytes(bytes: number) {
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

export function DownloadBadge({ children }: { children: ReactNode }) {
  return (
    <div className="inline-flex items-center gap-2 rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
      {children}
    </div>
  );
}

export function PrimaryLink({
  href,
  children,
}: {
  href: string;
  children: ReactNode;
}) {
  return (
    <Link
      href={href}
      className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
    >
      {children}
    </Link>
  );
}

export function SecondaryLink({
  href,
  children,
}: {
  href: string;
  children: ReactNode;
}) {
  return (
    <Link
      href={href}
      className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
    >
      {children}
    </Link>
  );
}

export function LatestReleasePanel({
  heading,
  productName,
  release,
  emptyMessage,
}: {
  heading: string;
  productName: string;
  release: DistributionReleaseResponse | null;
  emptyMessage: string;
}) {
  return (
    <div className="space-y-4 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
      <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
        {heading}
      </p>
      <div className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
        <p className="text-sm font-semibold text-[var(--atlas-ink)]">{productName}</p>
        <p className="text-xs text-[var(--atlas-ink-muted)]">
          {release ? `Latest stable v${release.version}` : "No stable release yet"} ·{" "}
          {formatDate(release?.published_at)}
        </p>
        <p className="mt-3 text-xs text-[var(--atlas-ink-muted)]">
          {release ? "Ready to download now." : emptyMessage}
        </p>
      </div>
    </div>
  );
}

export function PlatformDownloadCards({
  groups,
  emptyLabel,
}: {
  groups: ProductPlatformGroup[];
  emptyLabel: string;
}) {
  return (
    <section className="grid gap-6 lg:grid-cols-3">
      {groups.map((platform) => (
        <div key={platform.id} className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <p className="text-sm font-semibold text-[var(--atlas-ink)]">{platform.label}</p>
          <p className="text-xs text-[var(--atlas-ink-muted)]">{platform.detail}</p>
          <div className="mt-4 flex flex-col gap-3 text-sm">
            {platform.assets.length ? (
              platform.assets.map((asset) => (
                <a
                  key={asset.id}
                  href={`/api/v1/download/${asset.downloadId}`}
                  className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                  rel="noreferrer"
                  target="_blank"
                >
                  <span>
                    <span className="block font-medium">{asset.label}</span>
                    <span className="block text-xs text-[var(--atlas-ink-muted)]">{asset.detail}</span>
                  </span>
                  <span className="text-xs text-[var(--atlas-ink-muted)]">{formatBytes(asset.size)}</span>
                </a>
              ))
            ) : (
              <span className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
                {emptyLabel}
              </span>
            )}
          </div>
        </div>
      ))}
    </section>
  );
}
