import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { normalizeDownloadArch, normalizeDownloadOs } from "@/lib/download-target";
import { resolveRelease } from "@/lib/distribution";
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
  const release = await resolveRelease({
    product: "launcher",
    os,
    arch,
    channel: "stable",
  });
  if (!release) {
    return NextResponse.json({ error: "No launcher release found." }, { status: 404 });
  }

  const asset =
    release.assets.find((entry) => entry.kind === "installer") ??
    release.assets.find((entry) => entry.kind === "binary") ??
    null;
  if (!asset) {
    return NextResponse.json(
      { error: `No launcher installer found for ${os}/${arch}.` },
      { status: 404 },
    );
  }

  const location = `/api/v1/download/${asset.download_id}`;

  const headers = new Headers({ location });
  applyRateLimitHeaders(headers, limiter);
  headers.set("cache-control", "public, max-age=300");
  return new NextResponse(null, { status: 302, headers });
}
