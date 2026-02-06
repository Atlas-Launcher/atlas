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

  const authHeaders = getAuthHeaders();
  const hasAuthToken =
    authHeaders instanceof Headers
      ? authHeaders.has("Authorization")
      : Array.isArray(authHeaders)
        ? authHeaders.some(([key]) => key.toLowerCase() === "authorization")
        : Boolean((authHeaders as Record<string, string | undefined>).Authorization);

  const range = request.headers.get("range");
  let response = await fetchAssetWithRetries(asset.browser_download_url, {
    headers: {
      "User-Agent": "atlas-hub-downloads",
      ...authHeaders,
      ...(range ? { Range: range } : {}),
    },
    cache: "no-store",
  });

  // Private repos may return 404 on browser_download_url; retry through the asset API.
  if (
    !response.ok &&
    response.status === 404 &&
    hasAuthToken &&
    typeof asset.id === "number" &&
    getReleaseRepo()
  ) {
    response = await fetchAssetWithRetries(
      `https://api.github.com/repos/${getReleaseRepo()}/releases/assets/${asset.id}`,
      {
        headers: {
          "User-Agent": "atlas-hub-downloads",
          Accept: "application/octet-stream",
          ...authHeaders,
          ...(range ? { Range: range } : {}),
        },
        cache: "no-store",
      }
    );
  }

  // If auth headers are misconfigured, retry once without auth for public assets.
  if (!response.ok && hasAuthToken) {
    response = await fetchAssetWithRetries(
      asset.browser_download_url,
      {
        headers: {
          "User-Agent": "atlas-hub-downloads",
          ...(range ? { Range: range } : {}),
        },
        cache: "no-store",
      },
      2
    );
  }

  if (!response.ok) {
    if (!hasAuthToken || response.status === 401 || response.status === 403) {
      const headers = new Headers({ location: asset.browser_download_url });
      applyRateLimitHeaders(headers, limiter);
      headers.set("cache-control", "public, max-age=300");
      return new NextResponse(null, { status: 302, headers });
    }

    return NextResponse.json(
      { error: "Failed to fetch asset.", upstreamStatus: response.status },
      { status: 502 }
    );
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
