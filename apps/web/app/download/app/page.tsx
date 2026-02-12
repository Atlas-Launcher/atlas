import type { Metadata } from "next";
import { headers } from "next/headers";

import { PlatformGuidedDownload, type GuidedManualDownload, type GuidedPlatform } from "@/app/download/_components/platform-guided-download";
import { resolvePlatformReleases } from "@/app/download/_components/release-data";
import { formatDate, type ProductPlatformTarget } from "@/app/download/_components/shared";
import { detectDownloadTarget } from "@/lib/download-target";

export const metadata: Metadata = {
  title: "Launcher Download | Atlas Hub",
  description: "Download Atlas Launcher with guided install steps.",
};

const platformTargets: readonly ProductPlatformTarget[] = [
  {
    id: "windows",
    label: "Windows",
    detail: "Windows 10+",
    os: "windows",
    arches: ["x64"],
  },
  {
    id: "macos",
    label: "macOS",
    detail: "Apple silicon + Intel",
    os: "macos",
    arches: ["arm64", "x64"],
  },
  {
    id: "linux",
    label: "Linux",
    detail: "x64 + arm64",
    os: "linux",
    arches: ["x64", "arm64"],
  },
];

function archLabel(value: "x64" | "arm64") {
  return value === "arm64" ? "ARM64" : "x64";
}

function manualDownloadsForOs(
  releases: Awaited<ReturnType<typeof resolvePlatformReleases>>,
  os: "windows" | "macos" | "linux",
): GuidedManualDownload[] {
  return releases
    .filter((entry) => entry.os === os && entry.release)
    .flatMap((entry) => {
      const release = entry.release;
      if (!release) return [];

      return release.assets
        .filter((asset) => asset.kind === "installer" || asset.kind === "binary")
        .map((asset) => ({
          id: `${entry.key}:${asset.download_id}`,
          label: asset.kind === "installer" ? `${archLabel(entry.arch)} installer` : `${archLabel(entry.arch)} portable build`,
          detail: asset.filename,
          href: `/api/v1/download/${asset.download_id}`,
          size: asset.size,
        }));
    });
}

export default async function LauncherDownloadPage() {
  const requestHeaders = await headers();
  const detectedTarget = detectDownloadTarget(requestHeaders);
  const defaultPlatformId =
    detectedTarget?.os === "windows" || detectedTarget?.os === "macos" || detectedTarget?.os === "linux"
      ? detectedTarget.os
      : "windows";

  const releases = await resolvePlatformReleases("launcher", platformTargets);
  const firstRelease = releases.find((entry) => entry.release)?.release ?? null;

  const platforms: GuidedPlatform[] = [
    {
      id: "windows",
      label: "Windows",
      detail: "Desktop installer",
      action: {
        type: "download",
        label: "Download launcher for Windows",
        href: "/download/app/installer/latest/windows/x64",
        note: "Requires Windows 10 or newer.",
      },
      installTitle: "Install Atlas Launcher on Windows",
      installSteps: [
        "Download the Windows installer.",
        "Run setup and complete installation.",
        "Open Atlas Launcher and sign in.",
      ],
      manualTitle: "Manual Windows files",
      manualDownloads: manualDownloadsForOs(releases, "windows"),
      manualEmptyLabel: "No stable Windows launcher files available yet.",
      nextSteps: [
        "Sign in and connect your account.",
        "Choose a pack and press Play.",
        "Use stable channel builds for everyday play.",
      ],
    },
    {
      id: "macos",
      label: "macOS",
      detail: "Apple silicon + Intel",
      action: {
        type: "download",
        label: "Download launcher for macOS",
        href: "/download/app/installer/latest/macos/arm64",
        note: "Apple silicon is selected by default. Intel builds are in manual files.",
      },
      installTitle: "Install Atlas Launcher on macOS",
      installSteps: [
        "Download the macOS build.",
        "Open the installer package or app bundle.",
        "Launch Atlas and sign in.",
      ],
      manualTitle: "Manual macOS files",
      manualDownloads: manualDownloadsForOs(releases, "macos"),
      manualEmptyLabel: "No stable macOS launcher files available yet.",
      nextSteps: [
        "Allow app execution in system settings if prompted.",
        "Sign in and open your pack library.",
        "Keep auto-updates enabled for stable installs.",
      ],
    },
    {
      id: "linux",
      label: "Linux",
      detail: "x64 + arm64",
      action: {
        type: "download",
        label: "Download launcher for Linux",
        href: "/download/app/installer/latest/linux/x64",
        note: "Linux stable builds vary by distro and architecture.",
      },
      installTitle: "Install Atlas Launcher on Linux",
      installSteps: [
        "Download the Linux package for your architecture.",
        "Install with your distro package tools.",
        "Open Atlas Launcher and sign in.",
      ],
      manualTitle: "Manual Linux files",
      manualDownloads: manualDownloadsForOs(releases, "linux"),
      manualEmptyLabel: "No stable Linux launcher files available yet.",
      nextSteps: [
        "If needed, pick arm64 files for ARM VPS/devices.",
        "Run Atlas Launcher and authenticate.",
        "Select a pack and start playing.",
      ],
    },
  ];

  return (
    <PlatformGuidedDownload
      badge="Atlas Launcher"
      title="Download Atlas Launcher"
      subtitle="Choose your platform, use the recommended download, then follow the install steps below."
      latestLabel={
        firstRelease
          ? `Latest stable v${firstRelease.version} • ${formatDate(firstRelease.published_at)}`
          : "No stable launcher release published yet"
      }
      platforms={platforms}
      defaultPlatformId={defaultPlatformId}
    />
  );
}
