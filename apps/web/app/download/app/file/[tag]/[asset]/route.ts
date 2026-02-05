import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { getAuthHeaders, getReleaseByTag } from "@/lib/releases";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

function buildHeaders(contentType: string | null, size?: number, filename?: string) {
  const headers = new Headers();
  headers.set("cache-control", "public, max-age=31536000, immutable");
  headers.set("content-type", contentType ?? "application/octet-stream");
  if (size && Number.isFinite(size)) {
    headers.set("content-length", `${size}`);
  }
  if (filename) {
    headers.set("content-disposition", `attachment; filename="${filename}"`);
  }
  return headers;
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ tag: string; asset: string }> },
) {
  const { tag, asset } = await params;
  const limiter = rateLimit({
    id: `download-app:${getClientIp(request)}`,
    limit: 120,
    windowMs: 60_000,
  });

  if (!limiter.allowed) {
    const headers = buildHeaders("application/json");
    applyRateLimitHeaders(headers, limiter);
    return NextResponse.json({ error: "Too many download requests." }, { status: 429, headers });
  }

  const release = await getReleaseByTag(tag);
  if (!release) {
    return NextResponse.json({ error: "Release not found." }, { status: 404 });
  }

  const assetName = asset;
  const asset =
    release.assets.find((item) => item.name === assetName) ??
    release.assets.find((item) => item.name === decodeURIComponent(assetName));

  if (!asset) {
    return NextResponse.json({ error: "Asset not found." }, { status: 404 });
  }

  const range = request.headers.get("range");
  const response = await fetch(asset.browser_download_url, {
    headers: {
      "User-Agent": "atlas-hub-downloads",
      ...getAuthHeaders(),
      ...(range ? { Range: range } : {}),
    },
    next: { revalidate: 300 },
  });

  if (!response.ok) {
    return NextResponse.json({ error: "Failed to fetch asset." }, { status: 502 });
  }

  const headers = buildHeaders(
    response.headers.get("content-type") ?? asset.content_type,
    asset.size,
    asset.name,
  );
  applyRateLimitHeaders(headers, limiter);

  const contentRange = response.headers.get("content-range");
  if (contentRange) {
    headers.set("content-range", contentRange);
  }
  const acceptRanges = response.headers.get("accept-ranges");
  if (acceptRanges) {
    headers.set("accept-ranges", acceptRanges);
  }

  return new NextResponse(response.body, {
    status: response.status,
    headers,
  });
}
