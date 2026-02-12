import type { Metadata } from "next";
import { headers } from "next/headers";

import { DownloadHubSelector } from "@/app/download/_components/download-hub-selector";
import { resolveLatestReleaseForProduct } from "@/app/download/_components/release-data";
import { formatDate } from "@/app/download/_components/shared";
import { detectDownloadTarget } from "@/lib/download-target";
import { isDistributionArch } from "@/lib/distribution";

export const metadata: Metadata = {
  title: "Downloads | Atlas Hub",
  description: "Download Atlas Launcher, CLI, and Runner with guided setup.",
};

function releaseLabel(version?: string, publishedAt?: string) {
  if (!version) {
    return "No stable release yet";
  }
  return `Latest stable v${version} • ${formatDate(publishedAt)}`;
}

export default async function DownloadPage() {
  const requestHeaders = await headers();
  const detectedTarget = detectDownloadTarget(requestHeaders);
  const detectedArch =
    detectedTarget && isDistributionArch(detectedTarget.arch) ? detectedTarget.arch : "x64";

  const [launcherRelease, cliRelease, runnerRelease] = await Promise.all([
    resolveLatestReleaseForProduct("launcher"),
    resolveLatestReleaseForProduct("cli"),
    resolveLatestReleaseForProduct("runner", [
      { os: "linux", arch: "x64" },
      { os: "linux", arch: "arm64" },
    ]),
  ]);

  const launcherRecommendedHref = detectedTarget
    ? `/download/app/installer/latest/${detectedTarget.os}/${detectedArch}`
    : "/download/app/installer/latest";
  const cliRecommendedHref = detectedTarget
    ? `/download/cli/installer/latest/${detectedTarget.os}/${detectedArch}`
    : "/download/cli/installer/latest";

  const products = [
    {
      id: "launcher",
      label: "Launcher",
      detail: "Best for players who want a desktop app with automatic updates.",
      latest: releaseLabel(launcherRelease?.version, launcherRelease?.published_at),
      primaryHref: launcherRecommendedHref,
      primaryLabel: "Download launcher",
      pageHref: "/download/app",
      pageLabel: "View launcher options",
      installSteps: [
        "Download the launcher installer for your platform.",
        "Run setup and sign in with your Atlas account.",
        "Select a pack and start playing.",
      ],
    },
    {
      id: "cli",
      label: "CLI",
      detail: "Best for creators, automation, and CI pipelines.",
      latest: releaseLabel(cliRelease?.version, cliRelease?.published_at),
      primaryHref: cliRecommendedHref,
      primaryLabel: "Download CLI",
      pageHref: "/download/cli",
      pageLabel: "View CLI options",
      installSteps: [
        "Install the CLI for your platform.",
        "Verify with `atlas --version`.",
        "Run `atlas build` and `atlas publish` in your pack repo.",
      ],
    },
    {
      id: "runner",
      label: "Runner",
      detail: "Best for Linux VPS and server hosts. macOS manual, Windows via WSL.",
      latest: releaseLabel(runnerRelease?.version, runnerRelease?.published_at),
      primaryHref: "/download/runner",
      primaryLabel: "Open runner setup",
      pageHref: "/download/runner",
      pageLabel: "View runner options",
      installSteps: [
        "Use the Linux one-command install for VPS hosts.",
        "For macOS, download runner and runnerd manually.",
        "For Windows, use WSL and follow the Linux flow.",
      ],
    },
  ];

  return <DownloadHubSelector products={products} defaultProductId="launcher" />;
}
