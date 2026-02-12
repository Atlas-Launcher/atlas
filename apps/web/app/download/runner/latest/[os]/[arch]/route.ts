import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { normalizeDownloadArch, normalizeDownloadOs } from "@/lib/download-target";
import { isDistributionArch, resolveRelease } from "@/lib/distribution";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ os: string; arch: string }> },
) {
  const { os: osInput, arch: archInput } = await params;
  const limiter = rateLimit({
    id: `download-runner-latest:${getClientIp(request)}`,
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

  if (os !== "linux") {
    return NextResponse.json(
      { error: "Atlas Runner quick downloads are currently Linux-only." },
      { status: 400 },
    );
  }

  const arch = normalizeDownloadArch(archInput);
  if (!isDistributionArch(arch)) {
    return NextResponse.json({ error: `Unsupported architecture: ${archInput}.` }, { status: 400 });
  }

  const release = await resolveRelease({
    product: "runner",
    os,
    arch,
    channel: "stable",
  });

  if (!release) {
    return NextResponse.json({ error: "No runner release found." }, { status: 404 });
  }

  const asset =
    release.assets.find((entry) => entry.kind === "binary") ??
    release.assets.find((entry) => entry.kind === "installer") ??
    null;

  if (!asset) {
    return NextResponse.json(
      { error: `No runner binary found for ${os}/${arch}.` },
      { status: 404 },
    );
  }

  const location = `/api/v1/download/${asset.download_id}`;
  const headers = new Headers({ location });
  applyRateLimitHeaders(headers, limiter);
  headers.set("cache-control", "public, max-age=300");
  return new NextResponse(null, { status: 302, headers });
}
