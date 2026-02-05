type RateLimitEntry = {
  count: number;
  resetAt: number;
};

export type RateLimitResult = {
  allowed: boolean;
  remaining: number;
  resetAt: number;
  limit: number;
};

type RateLimitConfig = {
  id: string;
  limit: number;
  windowMs: number;
};

const STORE_KEY = "__atlas_rate_limit_store__";
const MAX_STORE_SIZE = 5000;

function getStore(): Map<string, RateLimitEntry> {
  const existing = (globalThis as typeof globalThis & {
    [STORE_KEY]?: Map<string, RateLimitEntry>;
  })[STORE_KEY];

  if (existing) {
    return existing;
  }

  const store = new Map<string, RateLimitEntry>();
  (globalThis as typeof globalThis & {
    [STORE_KEY]?: Map<string, RateLimitEntry>;
  })[STORE_KEY] = store;
  return store;
}

function cleanupStore(store: Map<string, RateLimitEntry>, now: number) {
  if (store.size <= MAX_STORE_SIZE) {
    return;
  }

  for (const [key, entry] of store.entries()) {
    if (entry.resetAt <= now) {
      store.delete(key);
    }
  }
}

export function getClientIp(request: Request): string {
  const headers = request.headers;
  const ipHeader =
    headers.get("cf-connecting-ip") ??
    headers.get("true-client-ip") ??
    headers.get("x-forwarded-for") ??
    headers.get("x-real-ip");

  if (!ipHeader) {
    return "unknown";
  }

  return ipHeader.split(",")[0]?.trim() || "unknown";
}

export function rateLimit({ id, limit, windowMs }: RateLimitConfig): RateLimitResult {
  const store = getStore();
  const now = Date.now();
  cleanupStore(store, now);

  const existing = store.get(id);
  if (!existing || existing.resetAt <= now) {
    const entry = { count: 1, resetAt: now + windowMs };
    store.set(id, entry);
    return {
      allowed: true,
      remaining: Math.max(0, limit - entry.count),
      resetAt: entry.resetAt,
      limit,
    };
  }

  existing.count += 1;
  store.set(id, existing);

  return {
    allowed: existing.count <= limit,
    remaining: Math.max(0, limit - existing.count),
    resetAt: existing.resetAt,
    limit,
  };
}

export function applyRateLimitHeaders(headers: Headers, result: RateLimitResult) {
  headers.set("x-ratelimit-limit", `${result.limit}`);
  headers.set("x-ratelimit-remaining", `${result.remaining}`);
  headers.set("x-ratelimit-reset", `${Math.ceil(result.resetAt / 1000)}`);
  if (!result.allowed) {
    const retryAfter = Math.max(0, Math.ceil((result.resetAt - Date.now()) / 1000));
    headers.set("retry-after", `${retryAfter}`);
  }
}
