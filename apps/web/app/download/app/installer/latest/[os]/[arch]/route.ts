import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { normalizeDownloadArch, normalizeDownloadOs } from "@/lib/download-target";
import { pickLauncherInstallerAsset } from "@/lib/installer-assets";
import { getLatestRelease } from "@/lib/releases";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ os: string; arch: string }> },
) {
  const { os: osInput, arch: archInput } = await params;
  const limiter = rateLimit({
    id: `download-launcher-installer-latest:${getClientIp(request)}`,
    limit: 120,
    windowMs: 60_000,
  });

  if (!limiter.allowed) {
    const headers = new Headers();
    applyRateLimitHeaders(headers, limiter);
    return NextResponse.json({ error: "Too many download requests." }, { status: 429, headers });
  }

  const os = normalizeDownloadOs(osInput);
  if (!os) {
    return NextResponse.json({ error: `Unsupported OS: ${osInput}.` }, { status: 400 });
  }

  const arch = normalizeDownloadArch(archInput);
  const release = await getLatestRelease("launcher-v");
  if (!release) {
    return NextResponse.json({ error: "No launcher release found." }, { status: 404 });
  }

  const asset = pickLauncherInstallerAsset(release.assets ?? [], os, arch);
  if (!asset) {
    return NextResponse.json(
      { error: `No launcher installer found for ${os}/${arch}.` },
      { status: 404 },
    );
  }

  const tag = encodeURIComponent(release.tag_name);
  const encodedAsset = encodeURIComponent(asset.name);
  const location = `/download/app/installer/file/${tag}/${encodedAsset}`;

  const headers = new Headers({ location });
  applyRateLimitHeaders(headers, limiter);
  headers.set("cache-control", "public, max-age=300");
  return new NextResponse(null, { status: 302, headers });
}
