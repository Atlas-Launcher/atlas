import type { Metadata } from "next";
import { headers } from "next/headers";

import { PlatformGuidedDownload, type GuidedManualDownload, type GuidedPlatform } from "@/app/download/_components/platform-guided-download";
import { resolvePlatformReleases } from "@/app/download/_components/release-data";
import { formatDate, type ProductPlatformTarget } from "@/app/download/_components/shared";
import { detectDownloadTarget } from "@/lib/download-target";

export const metadata: Metadata = {
  title: "CLI Download | Atlas Hub",
  description: "Download Atlas CLI with platform-guided install steps.",
};

const platformTargets: readonly ProductPlatformTarget[] = [
  {
    id: "windows",
    label: "Windows",
    detail: "PowerShell + cmd",
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
          label: asset.kind === "installer" ? `${archLabel(entry.arch)} installer package` : `${archLabel(entry.arch)} CLI binary`,
          detail: asset.filename,
          href: `/api/v1/download/${asset.download_id}`,
          size: asset.size,
        }));
    });
}

export default async function CliDownloadPage() {
  const requestHeaders = await headers();
  const detectedTarget = detectDownloadTarget(requestHeaders);
  const defaultPlatformId =
    detectedTarget?.os === "windows" || detectedTarget?.os === "macos" || detectedTarget?.os === "linux"
      ? detectedTarget.os
      : "linux";

  const releases = await resolvePlatformReleases("cli", platformTargets);
  const firstRelease = releases.find((entry) => entry.release)?.release ?? null;

  const platforms: GuidedPlatform[] = [
    {
      id: "windows",
      label: "Windows",
      detail: "PowerShell + cmd",
      action: {
        type: "download",
        label: "Download CLI for Windows",
        href: "/download/cli/installer/latest/windows/x64",
        note: "Use this for local creator workflows on Windows.",
      },
      installTitle: "Install Atlas CLI on Windows",
      installSteps: [
        "Download the CLI installer bundle.",
        "Run the install script or add atlas.exe to PATH.",
        "Verify with `atlas --version`.",
      ],
      manualTitle: "Manual Windows files",
      manualDownloads: manualDownloadsForOs(releases, "windows"),
      manualEmptyLabel: "No stable Windows CLI files available yet.",
      nextSteps: [
        "Run `atlas build --channel dev` in your pack repo.",
        "Use `atlas deploy` to publish builds.",
        "Store deploy tokens in CI secrets.",
      ],
    },
    {
      id: "macos",
      label: "macOS",
      detail: "Apple silicon + Intel",
      action: {
        type: "download",
        label: "Download CLI for macOS",
        href: "/download/cli/installer/latest/macos/arm64",
        note: "Apple silicon is selected by default. Intel files are listed below.",
      },
      installTitle: "Install Atlas CLI on macOS",
      installSteps: [
        "Download the macOS package or binary.",
        "Install and ensure `atlas` is on your PATH.",
        "Verify with `atlas --version`.",
      ],
      manualTitle: "Manual macOS files",
      manualDownloads: manualDownloadsForOs(releases, "macos"),
      manualEmptyLabel: "No stable macOS CLI files available yet.",
      nextSteps: [
        "Authenticate with your Atlas Hub.",
        "Run build and deploy commands from your repo root.",
        "Use CI workflow templates for automation.",
      ],
    },
    {
      id: "linux",
      label: "Linux",
      detail: "x64 + arm64",
      action: {
        type: "download",
        label: "Download CLI for Linux",
        href: "/download/cli/installer/latest/linux/x64",
        note: "Choose architecture-specific files from manual downloads as needed.",
      },
      installTitle: "Install Atlas CLI on Linux",
      installSteps: [
        "Download the package or binary for your distro/architecture.",
        "Install and ensure `atlas` is on your PATH.",
        "Verify with `atlas --version`.",
      ],
      manualTitle: "Manual Linux files",
      manualDownloads: manualDownloadsForOs(releases, "linux"),
      manualEmptyLabel: "No stable Linux CLI files available yet.",
      nextSteps: [
        "Run `atlas build --channel dev`.",
        "Publish artifacts with `atlas deploy`.",
        "Integrate into CI using the atlas-build workflow.",
      ],
    },
  ];

  return (
    <PlatformGuidedDownload
      badge="Atlas CLI"
      title="Download Atlas CLI"
      subtitle="Select your platform, install the CLI, and run your build/deploy workflow."
      latestLabel={
        firstRelease
          ? `Latest stable v${firstRelease.version} • ${formatDate(firstRelease.published_at)}`
          : "No stable CLI release published yet"
      }
      platforms={platforms}
      defaultPlatformId={defaultPlatformId}
    />
  );
}
