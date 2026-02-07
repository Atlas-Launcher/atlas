import type { ReleaseAsset } from "@/lib/releases";

import { getArchNeedles, type DownloadOs } from "@/lib/download-target";

const CLI_PRIORITIES: Record<DownloadOs, string[]> = {
  windows: [".msi", ".exe", ".zip"],
  macos: [".pkg", ".dmg", ".zip", ".tar.gz"],
  linux: [".deb", ".rpm", ".appimage", ".tar.gz", ".zip"],
};

const LAUNCHER_PRIORITIES: Record<DownloadOs, string[]> = {
  windows: [".msi", ".exe", ".nsis.zip", ".msi.zip", ".zip"],
  macos: [".dmg", ".pkg", ".app.tar.gz", ".zip", ".tar.gz"],
  linux: [".appimage", ".deb", ".rpm", ".tar.gz", ".zip"],
};

const CLI_OS_NEEDLES: Record<DownloadOs, string[]> = {
  windows: ["windows", "win32", "win64"],
  macos: ["macos", "darwin", "osx"],
  linux: ["linux"],
};

function isIgnoredAsset(assetName: string) {
  const name = assetName.toLowerCase();
  return name.endsWith(".sig") || name.startsWith("source code");
}

function matchesPriority(name: string, priority: string) {
  return name.endsWith(priority);
}

function scorePriority(name: string, priorities: string[]) {
  const index = priorities.findIndex((priority) => matchesPriority(name, priority));
  return index === -1 ? priorities.length : index;
}

function filterByArch(assets: ReleaseAsset[], arch: string) {
  const needles = getArchNeedles(arch);
  const matches = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    return needles.some((needle) => name.includes(needle));
  });
  return matches.length ? matches : assets;
}

function sortByPriority(assets: ReleaseAsset[], priorities: string[]) {
  return [...assets].sort((a, b) => {
    const aName = a.name.toLowerCase();
    const bName = b.name.toLowerCase();
    const priorityDiff = scorePriority(aName, priorities) - scorePriority(bName, priorities);
    if (priorityDiff !== 0) return priorityDiff;
    return aName.localeCompare(bName);
  });
}

export function pickCliInstallerAsset(
  assets: ReleaseAsset[],
  os: DownloadOs,
  arch: string,
): ReleaseAsset | null {
  const priorities = CLI_PRIORITIES[os];
  const osNeedles = CLI_OS_NEEDLES[os];

  const installerCandidates = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    if (isIgnoredAsset(name)) return false;
    if (!priorities.some((priority) => matchesPriority(name, priority))) return false;
    if (!osNeedles.some((needle) => name.includes(needle))) return false;
    return name.includes("installer");
  });

  if (installerCandidates.length) {
    const scoped = filterByArch(installerCandidates, arch);
    return sortByPriority(scoped, priorities)[0] ?? null;
  }

  // Fallback to legacy non-installer CLI artifacts so existing releases still resolve.
  const fallbackCandidates = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    if (isIgnoredAsset(name)) return false;
    if (!priorities.some((priority) => matchesPriority(name, priority))) return false;
    return osNeedles.some((needle) => name.includes(needle));
  });

  if (!fallbackCandidates.length) {
    return null;
  }

  const scoped = filterByArch(fallbackCandidates, arch);
  return sortByPriority(scoped, priorities)[0] ?? null;
}

export function pickLauncherInstallerAsset(
  assets: ReleaseAsset[],
  os: DownloadOs,
  arch: string,
): ReleaseAsset | null {
  const priorities = LAUNCHER_PRIORITIES[os];
  const candidates = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    if (isIgnoredAsset(name)) return false;
    return priorities.some((priority) => matchesPriority(name, priority));
  });

  if (!candidates.length) {
    return null;
  }

  const scoped = filterByArch(candidates, arch);
  return sortByPriority(scoped, priorities)[0] ?? null;
}
