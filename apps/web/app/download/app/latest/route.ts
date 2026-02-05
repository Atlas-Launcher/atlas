import { NextResponse } from "next/server";

import { getAuthHeaders, getLatestRelease } from "@/lib/releases";
import { applyRateLimitHeaders, getClientIp, rateLimit } from "@/lib/rate-limit";

export async function GET(request: Request) {
  const limiter = rateLimit({
    id: `latest-manifest:${getClientIp(request)}`,
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

  const updateAsset = release.assets.find((asset) => {
    const name = asset.name.toLowerCase();
    return name.includes("latest") && name.endsWith(".json");
  });

  if (!updateAsset) {
    return NextResponse.json(
      { error: "No update manifest found for the latest launcher release." },
      { status: 404 },
    );
  }

  const response = await fetch(updateAsset.browser_download_url, {
    headers: {
      "User-Agent": "atlas-hub-downloads",
      ...getAuthHeaders(),
    },
    next: { revalidate: 300 },
  });

  if (!response.ok) {
    return NextResponse.json({ error: "Failed to fetch update manifest." }, { status: 502 });
  }

  const body = await response.text();
  return new NextResponse(body, {
    headers: {
      "content-type": "application/json",
      "cache-control": "public, max-age=300",
    },
  });
}
