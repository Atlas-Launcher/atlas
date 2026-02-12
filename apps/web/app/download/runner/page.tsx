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

  const installCommand = `curl -fsSL ${hubOrigin}/download/runner/install | sudo bash -s --`;
  const wslCommand = `wsl -e bash -lc 'curl -fsSL ${hubOrigin}/download/runner/install | sudo bash -s -- --no-daemon-install'`;

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
      detail: "Recommended for servers",
      action: {
        type: "command",
        label: "Install with one command",
        command: installCommand,
        note: "Installs Atlas Runner and has it start when your system does.",
      },
      installTitle: "Install Atlas Runner on Linux",
      installSteps: [
        "Open your server terminal.",
        "Run the install command above.",
        "Verify: `atlas-runner --version`.",
        "Link: `atlas-runner auth login`.",
        "Start: `atlas-runner server start`.",
      ],
      manualTitle: "Manual Linux downloads (runner + runnerd)",
      manualDownloads: linuxManual,
      manualEmptyLabel: "No stable Linux builds available yet.",
      nextSteps: [
        "Run `atlas-runner auth login` to link this machine.",
        "Run `atlas-runner server start` to initialize and start the server.",
        "Use the stable channel for production systems.",
      ],
    },

    {
      id: "macos",
      label: "macOS",
      detail: "Manual install",
      action: {
        type: "info",
        label: "Download binaries",
        note: "Download both `atlas-runner` and `atlas-runnerd` for your architecture.",
      },
      installTitle: "Install Atlas Runner on macOS",
      installSteps: [
        "Download `atlas-runner` and `atlas-runnerd`.",
        "Move them into a folder in your PATH (for example `/usr/local/bin`).",
        "Make them executable: `chmod +x atlas-runner atlas-runnerd`.",
        "Verify: `atlas-runner --version`.",
        "Link: `atlas-runner auth login`",
        "Start: `atlas-runner server start`.",
      ],
      manualTitle: "Manual macOS downloads (runner + runnerd)",
      manualDownloads: macosManual,
      manualEmptyLabel: "No stable macOS builds available yet.",
      nextSteps: [
        "Keep runner and runnerd on the same version.",
        "Run `atlas-runner auth login` and then `atlas-runner server start`.",
        "Update both binaries together when upgrading.",
      ],
    },

    {
      id: "windows",
      label: "Windows (WSL)",
      detail: "Install Atlas Runner inside the Windows Subsystem for Linux (WSL)",
      action: {
        type: "command",
        label: "Install with one command",
        command: wslCommand,
        note: "WSL installs do not start with your system by default. For always-on hosts, prefer Linux install.",
      },
      installTitle: "Install Atlas Runner on Windows (WSL)",
      installSteps: [
        "Install WSL and open a Linux distro (Ubuntu, Debian, etc.).",
        "Run the install command on your Windows host.",
        "Verify in WSL: `atlas-runner --version`.",
        "Link the host in WSL: `atlas-runner auth login`.",
        "Apply setup in WSL: `atlas-runner server start`.",
      ],
      manualTitle: "Manual downloads",
      manualDownloads: [],
      manualEmptyLabel: "Use WSL and follow the Linux install flow.",
      nextSteps: [],
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
