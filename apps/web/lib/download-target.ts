export type DownloadOs = "windows" | "macos" | "linux";

const OS_ALIASES: Record<string, DownloadOs> = {
  windows: "windows",
  win32: "windows",
  win64: "windows",
  macos: "macos",
  darwin: "macos",
  osx: "macos",
  linux: "linux",
  android: "linux",
};

const ARCH_ALIASES: Record<string, string[]> = {
  x64: ["x64", "x86_64", "amd64"],
  x86_64: ["x64", "x86_64", "amd64"],
  amd64: ["x64", "x86_64", "amd64"],
  arm64: ["arm64", "aarch64"],
  aarch64: ["arm64", "aarch64"],
  x86: ["x86", "i686", "386"],
  i686: ["x86", "i686", "386"],
  "386": ["x86", "i686", "386"],
};

function cleanHeaderValue(value: string | null) {
  if (!value) return "";
  return value.trim().replace(/^["']+|["']+$/g, "");
}

function detectOsFromUserAgent(userAgent: string): DownloadOs | null {
  const ua = userAgent.toLowerCase();
  if (ua.includes("windows")) return "windows";
  if (ua.includes("mac os x") || ua.includes("macintosh") || ua.includes("darwin")) return "macos";
  if (ua.includes("linux") || ua.includes("x11") || ua.includes("android")) return "linux";
  return null;
}

function detectArchFromUserAgent(userAgent: string): string | null {
  const ua = userAgent.toLowerCase();
  if (
    ua.includes("aarch64") ||
    ua.includes("arm64") ||
    ua.includes("armv8") ||
    ua.includes("apple silicon")
  ) {
    return "arm64";
  }
  if (
    ua.includes("x86_64") ||
    ua.includes("amd64") ||
    ua.includes("win64") ||
    ua.includes("x64")
  ) {
    return "x64";
  }
  if (ua.includes("i686") || ua.includes("i386") || ua.includes(" x86")) {
    return "x86";
  }
  return null;
}

export function normalizeDownloadOs(value: string): DownloadOs | null {
  return OS_ALIASES[value.trim().toLowerCase()] ?? null;
}

export function normalizeDownloadArch(value: string): string {
  const normalized = value.trim().toLowerCase();
  const aliases = ARCH_ALIASES[normalized];
  return aliases ? aliases[0] : normalized || "x64";
}

export function getArchNeedles(value: string) {
  const normalized = normalizeDownloadArch(value);
  return ARCH_ALIASES[normalized] ?? [normalized];
}

export function detectDownloadTarget(
  headers: Headers,
): { os: DownloadOs; arch: string } | null {
  const platformHint = cleanHeaderValue(headers.get("sec-ch-ua-platform"));
  const archHint = cleanHeaderValue(headers.get("sec-ch-ua-arch"));
  const userAgent = headers.get("user-agent") ?? "";

  const os =
    normalizeDownloadOs(platformHint) ??
    (platformHint ? normalizeDownloadOs(platformHint.toLowerCase()) : null) ??
    detectOsFromUserAgent(userAgent);

  if (!os) {
    return null;
  }

  const arch = normalizeDownloadArch(archHint || detectArchFromUserAgent(userAgent) || "x64");
  return { os, arch };
}
