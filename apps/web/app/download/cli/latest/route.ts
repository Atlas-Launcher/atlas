import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { detectDownloadTarget } from "@/lib/download-target";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

export async function GET(request: NextRequest) {
  const limiter = rateLimit({
    id: `download-cli-binary-auto:${getClientIp(request)}`,
    limit: 120,
    windowMs: 60_000,
  });

  if (!limiter.allowed) {
    const headers = new Headers();
    applyRateLimitHeaders(headers, limiter);
    return NextResponse.json({ error: "Too many download requests." }, { status: 429, headers });
  }

  const target = detectDownloadTarget(request.headers);
  if (!target) {
    const fallback = new URL("/download/cli", request.url);
    const headers = new Headers({ location: fallback.toString() });
    applyRateLimitHeaders(headers, limiter);
    headers.set("cache-control", "no-store");
    return new NextResponse(null, { status: 302, headers });
  }

  const location = new URL(`/download/cli/latest/${target.os}/${target.arch}`, request.url);
  const headers = new Headers({ location: location.toString() });
  applyRateLimitHeaders(headers, limiter);
  headers.set("cache-control", "no-store");
  return new NextResponse(null, { status: 302, headers });
}
