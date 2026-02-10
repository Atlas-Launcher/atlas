import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

import { getAuthHeaders, getReleaseByTag, getReleaseRepo } from "@/lib/releases";
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

function sleep(ms: number) {
  return new Promise<void>((resolve) => {
    setTimeout(resolve, ms);
  });
}

function shouldRetryStatus(status: number) {
  return status === 429 || status >= 500;
}

async function fetchAssetWithRetries(
  url: string,
  init: RequestInit,
  attempts = 4
) {
  let response: Response | null = null;
  let lastError: unknown = null;

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      response = await fetch(url, init);
      if (response.ok || !shouldRetryStatus(response.status)) {
        return response;
      }
    } catch (error) {
      lastError = error;
    }

    if (attempt < attempts) {
      await sleep(250 * 2 ** (attempt - 1));
    }
  }

  if (response) {
    return response;
  }

  throw lastError instanceof Error ? lastError : new Error("Failed to fetch asset.");
}

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ tag: string; asset: string }> },
) {
  const { tag, asset: assetParam } = await params;
  const limiter = rateLimit({
    id: `download-cli:${getClientIp(request)}`,
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

  const assetName = assetParam;
  const asset =
    release.assets.find((item) => item.name === assetName) ??
    release.assets.find((item) => item.name === decodeURIComponent(assetName));

  if (!asset) {
    return NextResponse.json({ error: "Asset not found." }, { status: 404 });
  }

  const headers = buildHeaders(asset.content_type, asset.size, asset.name);
  applyRateLimitHeaders(headers, limiter);

  return NextResponse.redirect(asset.browser_download_url, { headers });
}
