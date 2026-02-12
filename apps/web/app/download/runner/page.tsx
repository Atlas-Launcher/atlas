import type { Metadata } from "next";
import { headers } from "next/headers";

import { PlatformGuidedDownload, type GuidedManualDownload, type GuidedPlatform } from "@/app/download/_components/platform-guided-download";
import { resolvePlatformReleases } from "@/app/download/_components/release-data";
import { formatDate, type ProductPlatformTarget } from "@/app/download/_components/shared";

export const metadata: Metadata = {
  title: "Runner Download | Atlas Hub",
  description: "Install Atlas Runner with Linux VPS-first flow, plus macOS and WSL guidance.",
};

const releaseTargets: readonly ProductPlatformTarget[] = [
  {
    id: "linux",
    label: "Linux",
    detail: "x64 + arm64",
    os: "linux",
    arches: ["x64", "arm64"],
  },
  {
    id: "macos",
    label: "macOS",
    detail: "Apple silicon + Intel",
    os: "macos",
    arches: ["arm64", "x64"],
  },
];

function archLabel(value: "x64" | "arm64") {
  return value === "arm64" ? "ARM64" : "x64";
}

function resolveHubOrigin(requestHost: string | null, requestProto: string | null) {
  const configured = process.env.NEXT_PUBLIC_BETTER_AUTH_URL?.trim();
  if (configured) {
    try {
      return new URL(configured).origin;
    } catch {
      return configured.replace(/\/+$/, "");
    }
  }

  if (requestHost) {
    return `${requestProto ?? "https"}://${requestHost}`;
  }

  return "https://localhost:3000";
}

function collectManualDownloads(
  runnerReleases: Awaited<ReturnType<typeof resolvePlatformReleases>>,
  runnerdReleases: Awaited<ReturnType<typeof resolvePlatformReleases>>,
  os: "linux" | "macos",
): GuidedManualDownload[] {
  return ["x64", "arm64"].flatMap((archValue) => {
    const arch = archValue as "x64" | "arm64";
    const runnerRelease = runnerReleases.find(
      (entry) => entry.os === os && entry.arch === arch,
    )?.release;
    const runnerdRelease = runnerdReleases.find(
      (entry) => entry.os === os && entry.arch === arch,
    )?.release;

    const runnerAsset = runnerRelease?.assets.find((asset) => asset.kind === "binary") ??
      runnerRelease?.assets.find((asset) => asset.kind === "installer") ??
      null;
    const runnerdAsset = runnerdRelease?.assets.find((asset) => asset.kind === "binary") ??
      runnerdRelease?.assets.find((asset) => asset.kind === "installer") ??
      null;

    const downloads: GuidedManualDownload[] = [];

    if (runnerAsset) {
      downloads.push({
        id: `runner-${os}-${arch}-${runnerAsset.download_id}`,
        label: `${archLabel(arch)} atlas-runner`,
        detail: runnerAsset.filename,
        href: `/api/v1/download/${runnerAsset.download_id}`,
        size: runnerAsset.size,
      });
    }

    if (runnerdAsset) {
      downloads.push({
        id: `runnerd-${os}-${arch}-${runnerdAsset.download_id}`,
        label: `${archLabel(arch)} atlas-runnerd`,
        detail: runnerdAsset.filename,
        href: `/api/v1/download/${runnerdAsset.download_id}`,
        size: runnerdAsset.size,
      });
    }

    return downloads;
  });
}

export default async function RunnerDownloadPage() {
  const requestHeaders = await headers();
  const hubOrigin = resolveHubOrigin(
    requestHeaders.get("x-forwarded-host") ?? requestHeaders.get("host"),
    requestHeaders.get("x-forwarded-proto"),
  );

  const installCommand = `curl -fsSL ${hubOrigin}/download/runner/install | sudo bash`;
  const wslCommand = `wsl -e bash -lc '${installCommand}'`;

  const [runnerReleases, runnerdReleases] = await Promise.all([
    resolvePlatformReleases("runner", releaseTargets),
    resolvePlatformReleases("runnerd", releaseTargets),
  ]);

  const latestRunner =
    runnerReleases.find((entry) => entry.os === "linux" && entry.release)?.release ??
    runnerReleases.find((entry) => entry.release)?.release ??
    null;

  const linuxManual = collectManualDownloads(runnerReleases, runnerdReleases, "linux");
  const macosManual = collectManualDownloads(runnerReleases, runnerdReleases, "macos");

  const platforms: GuidedPlatform[] = [
    {
      id: "linux",
      label: "Linux",
      detail: "Recommended for VPS",
      action: {
        type: "command",
        label: "Install with one command",
        command: installCommand,
        note: "Uses your Hub origin from NEXT_PUBLIC_BETTER_AUTH_URL.",
      },
      installTitle: "Install Atlas Runner on Linux VPS",
      installSteps: [
        "Run the one-command install in your VPS shell.",
        "Verify with `atlas-runner --version`.",
        "Continue with runner auth/install workflow for daemon setup.",
      ],
      manualTitle: "Manual Linux downloads (runner + runnerd)",
      manualDownloads: linuxManual,
      manualEmptyLabel: "No stable Linux runner artifacts available yet.",
      nextSteps: [
        "Run `atlas-runner auth` to connect your host.",
        "Use install/up commands to initialize runtime state.",
        "Use stable channel builds for production.",
      ],
    },
    {
      id: "macos",
      label: "macOS",
      detail: "Native manual install",
      action: {
        type: "info",
        label: "Manual download required",
        note: "Download both atlas-runner and atlas-runnerd below for your architecture.",
      },
      installTitle: "Install Atlas Runner on macOS",
      installSteps: [
        "Download both `atlas-runner` and `atlas-runnerd` binaries.",
        "Move binaries into your PATH and mark executable.",
        "Verify with `atlas-runner --version` and start runner workflow commands.",
      ],
      manualTitle: "Manual macOS downloads (runner + runnerd)",
      manualDownloads: macosManual,
      manualEmptyLabel: "No stable macOS runner artifacts available yet.",
      nextSteps: [
        "Keep runner and runnerd versions aligned.",
        "Run auth and install flows from atlas-runner.",
        "Track release updates on this page.",
      ],
    },
    {
      id: "windows",
      label: "Windows",
      detail: "Use WSL",
      action: {
        type: "command",
        label: "Run install in WSL",
        command: wslCommand,
        note: "Windows deployment is supported through WSL Linux environments.",
      },
      installTitle: "Install Atlas Runner on Windows (WSL)",
      installSteps: [
        "Install WSL and open a Linux distro shell.",
        "Run the WSL command shown above inside your distro.",
        "Verify in WSL with `atlas-runner --version`.",
      ],
      manualTitle: "Manual downloads",
      manualDownloads: [],
      manualEmptyLabel: "Use WSL and follow the Linux install flow.",
      nextSteps: [
        "Keep runner workloads inside Linux/WSL context.",
        "Use Linux binaries rather than native Windows execution.",
        "Follow Linux VPS docs for operational guidance.",
      ],
    },
  ];

  return (
    <PlatformGuidedDownload
      badge="Atlas Runner"
      title="Download Atlas Runner"
      subtitle="Linux VPS is the recommended path. macOS is supported with manual binaries, and Windows uses WSL."
      latestLabel={
        latestRunner
          ? `Latest stable v${latestRunner.version} • ${formatDate(latestRunner.published_at)}`
          : "No stable runner release published yet"
      }
      platforms={platforms}
      defaultPlatformId="linux"
    />
  );
}
