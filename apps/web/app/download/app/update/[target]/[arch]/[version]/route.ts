import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { getAuthHeaders, getLatestRelease, type ReleaseAsset } from "@/lib/releases";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

const TARGET_PRIORITIES: Record<string, string[]> = {
  windows: [".msi", ".exe"],
  darwin: [".app.tar.gz"],
  linux: [".appimage"],
};

function normalizeVersion(input: string): string {
  return input.trim().replace(/^v/, "");
}

function parseSemver(input: string) {
  const normalized = normalizeVersion(input);
  const [coreWithMeta, prerelease] = normalized.split("-", 2);
  const core = coreWithMeta.split("+", 1)[0];
  const parts = core.split(".");
  if (parts.length < 3) return null;
  const [major, minor, patch] = parts.map((part) => Number(part));
  if ([major, minor, patch].some((value) => Number.isNaN(value))) {
    return null;
  }
  return {
    major,
    minor,
    patch,
    prerelease: prerelease ?? null,
  };
}

function compareSemver(a: string, b: string) {
  const parsedA = parseSemver(a);
  const parsedB = parseSemver(b);
  if (!parsedA || !parsedB) {
    return null;
  }
  if (parsedA.major !== parsedB.major) return parsedA.major - parsedB.major;
  if (parsedA.minor !== parsedB.minor) return parsedA.minor - parsedB.minor;
  if (parsedA.patch !== parsedB.patch) return parsedA.patch - parsedB.patch;
  if (parsedA.prerelease && !parsedB.prerelease) return -1;
  if (!parsedA.prerelease && parsedB.prerelease) return 1;
  if (parsedA.prerelease && parsedB.prerelease) {
    return parsedA.prerelease.localeCompare(parsedB.prerelease);
  }
  return 0;
}

function pickUpdateAsset(assets: ReleaseAsset[], target: string, arch: string) {
  const priorities = TARGET_PRIORITIES[target] ?? [];
  if (!priorities.length) return null;

  const candidates = assets.filter((asset) => {
    const name = asset.name.toLowerCase();
    if (name.endsWith(".sig")) return false;
    return priorities.some((ext) => name.endsWith(ext));
  });

  if (!candidates.length) return null;

  const archNeedle = arch.toLowerCase();
  const archMatches = candidates.filter((asset) => asset.name.toLowerCase().includes(archNeedle));
  const scoped = archMatches.length ? archMatches : candidates;

  const sorted = [...scoped].sort((a, b) => {
    const aIndex = priorities.findIndex((ext) => a.name.toLowerCase().endsWith(ext));
    const bIndex = priorities.findIndex((ext) => b.name.toLowerCase().endsWith(ext));
    return (aIndex === -1 ? priorities.length : aIndex) - (bIndex === -1 ? priorities.length : bIndex);
  });

  return sorted[0] ?? null;
}

async function fetchSignature(asset: ReleaseAsset) {
  const response = await fetch(asset.browser_download_url, {
    headers: {
      "User-Agent": "atlas-hub-updater",
      ...getAuthHeaders(),
    },
    next: { revalidate: 300 },
  });

  if (!response.ok) {
    return null;
  }

  return (await response.text()).trim();
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ target: string; arch: string; version: string }> },
) {
  const { target, arch, version } = await params;
  const limiter = rateLimit({
    id: `updater:${getClientIp(request)}`,
    limit: 60,
    windowMs: 300_000,
  });

  if (!limiter.allowed) {
    const headers = new Headers();
    applyRateLimitHeaders(headers, limiter);
    return NextResponse.json({ error: "Too many update checks." }, { status: 429, headers });
  }

  const release = await getLatestRelease("launcher-v");
  if (!release) {
    return NextResponse.json({ error: "No launcher release found." }, { status: 404 });
  }

  const latestVersion = normalizeVersion(release.tag_name?.replace(/^launcher-v/, "") ?? "");
  if (!latestVersion) {
    return NextResponse.json({ error: "Latest release is missing a version tag." }, { status: 500 });
  }

  const compare = compareSemver(version, latestVersion);
  if (compare !== null && compare >= 0) {
    const headers = new Headers();
    applyRateLimitHeaders(headers, limiter);
    return new NextResponse(null, { status: 204, headers });
  }

  const updateAsset = pickUpdateAsset(release.assets ?? [], target, arch);
  if (!updateAsset) {
    return NextResponse.json(
      { error: `No update asset found for target ${target}.` },
      { status: 404 },
    );
  }

  const signatureAsset = release.assets.find(
    (asset) => asset.name.toLowerCase() === `${updateAsset.name.toLowerCase()}.sig`,
  );

  if (!signatureAsset) {
    return NextResponse.json({ error: "Missing update signature." }, { status: 404 });
  }

  const signature = await fetchSignature(signatureAsset);
  if (!signature) {
    return NextResponse.json({ error: "Failed to fetch update signature." }, { status: 502 });
  }

  const origin = new URL(request.url).origin;
  const tag = release.tag_name ?? "launcher-latest";
  const encodedTag = encodeURIComponent(tag);
  const encodedAsset = encodeURIComponent(updateAsset.name);
  const proxiedUrl = `${origin}/download/app/file/${encodedTag}/${encodedAsset}`;

  return NextResponse.json(
    {
      version: latestVersion,
      notes: release.name ?? "Atlas Launcher update",
      pub_date: release.published_at ?? release.created_at ?? new Date().toISOString(),
      url: proxiedUrl,
      signature,
    },
    {
      headers: {
        "cache-control": "public, max-age=300",
        "x-ratelimit-limit": `${limiter.limit}`,
        "x-ratelimit-remaining": `${limiter.remaining}`,
        "x-ratelimit-reset": `${Math.ceil(limiter.resetAt / 1000)}`,
      },
    },
  );
}
