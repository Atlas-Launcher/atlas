import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { getLatestRelease, type ReleaseAsset } from "@/lib/releases";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

const OS_ALIASES: Record<string, "linux" | "windows" | "macos"> = {
  linux: "linux",
  windows: "windows",
  win32: "windows",
  macos: "macos",
  darwin: "macos",
  osx: "macos",
};

const ARCH_ALIASES: Record<string, string[]> = {
  x64: ["x64", "x86_64", "amd64"],
  amd64: ["x64", "x86_64", "amd64"],
  x86_64: ["x64", "x86_64", "amd64"],
  arm64: ["arm64", "aarch64"],
  aarch64: ["arm64", "aarch64"],
  x86: ["x86", "i686", "386"],
  i686: ["x86", "i686", "386"],
  "386": ["x86", "i686", "386"],
};

function normalizeOs(value: string) {
  return OS_ALIASES[value.trim().toLowerCase()] ?? null;
}

function archNeedles(value: string) {
  const normalized = value.trim().toLowerCase();
  return ARCH_ALIASES[normalized] ?? [normalized];
}

function pickCliAsset(assets: ReleaseAsset[], os: string, arch: string) {
  const normalizedOs = normalizeOs(os);
  if (!normalizedOs) {
    return null;
  }

  const osNeedles =
    normalizedOs === "macos" ? ["-macos-", "-darwin-", "-osx-"] : [`-${normalizedOs}-`];
  const needles = archNeedles(arch);

  const candidates = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    if (name.endsWith(".sig")) return false;
    if (name.startsWith("source code")) return false;
    return osNeedles.some((needle) => name.includes(needle));
  });

  if (!candidates.length) {
    return null;
  }

  const archMatches = candidates.filter((asset) => {
    const name = asset.name.toLowerCase();
    return needles.some((needle) => name.includes(needle));
  });
  const scoped = archMatches.length ? archMatches : candidates;

  const sorted = [...scoped].sort((a, b) => {
    const aName = a.name.toLowerCase();
    const bName = b.name.toLowerCase();

    if (normalizedOs === "windows") {
      const aExe = aName.endsWith(".exe");
      const bExe = bName.endsWith(".exe");
      if (aExe !== bExe) return aExe ? -1 : 1;
    }

    return aName.localeCompare(bName);
  });

  return sorted[0] ?? null;
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ os: string; arch: string }> }
) {
  const { os, arch } = await params;
  const limiter = rateLimit({
    id: `download-cli-latest:${getClientIp(request)}`,
    limit: 120,
    windowMs: 60_000,
  });

  if (!limiter.allowed) {
    const headers = new Headers();
    applyRateLimitHeaders(headers, limiter);
    return NextResponse.json(
      { error: "Too many download requests." },
      { status: 429, headers }
    );
  }

  const release = await getLatestRelease("cli-v");
  if (!release) {
    return NextResponse.json({ error: "No CLI release found." }, { status: 404 });
  }

  const asset = pickCliAsset(release.assets ?? [], os, arch);
  if (!asset) {
    return NextResponse.json(
      { error: `No CLI asset found for ${os}/${arch}.` },
      { status: 404 }
    );
  }

  const tag = encodeURIComponent(release.tag_name);
  const encodedAsset = encodeURIComponent(asset.name);
  const location = `/download/cli/file/${tag}/${encodedAsset}`;

  const headers = new Headers({ location });
  applyRateLimitHeaders(headers, limiter);
  headers.set("cache-control", "public, max-age=300");
  return new NextResponse(null, { status: 302, headers });
}
