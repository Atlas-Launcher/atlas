const CURSEFORGE_API_BASE = "https://api.curseforge.com/v1";

export class CurseForgeProxyError extends Error {
  status: number;
  details: unknown;

  constructor(status: number, message: string, details: unknown) {
    super(message);
    this.status = status;
    this.details = details;
  }
}

function getCurseForgeApiKey(): string {
  const value = process.env.CURSEFORGE_API_KEY?.trim();
  if (!value) {
    throw new CurseForgeProxyError(
      503,
      "CurseForge API key is not configured.",
      { error: "Service unavailable" }
    );
  }
  return value;
}

function parseJsonSafe(text: string): unknown {
  if (!text.trim()) {
    return {};
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new CurseForgeProxyError(502, "Invalid response from CurseForge.", {
      error: "Bad gateway",
    });
  }
}

function buildUrl(path: string, params?: URLSearchParams) {
  const url = new URL(`${CURSEFORGE_API_BASE}${path}`);
  if (params) {
    for (const [key, value] of params.entries()) {
      url.searchParams.append(key, value);
    }
  }
  return url;
}

export async function curseForgeGet(path: string, params?: URLSearchParams): Promise<unknown> {
  const response = await fetch(buildUrl(path, params), {
    method: "GET",
    headers: {
      Accept: "application/json",
      "x-api-key": getCurseForgeApiKey(),
    },
    cache: "no-store",
  });

  const text = await response.text();
  const body = parseJsonSafe(text);
  if (!response.ok) {
    throw new CurseForgeProxyError(
      response.status,
      "CurseForge request failed.",
      body
    );
  }

  return body;
}

export function pickAllowedParams(
  params: URLSearchParams,
  allowed: readonly string[]
): URLSearchParams {
  const next = new URLSearchParams();
  const allowedSet = new Set(allowed);
  for (const [key, value] of params.entries()) {
    if (allowedSet.has(key)) {
      if (key === "pageSize") {
        const size = parseInt(value, 10);
        next.append(key, Math.min(isNaN(size) ? 50 : size, 100).toString()); // Cap at 100
      } else {
        next.append(key, value);
      }
    }
  }
  return next;
}
