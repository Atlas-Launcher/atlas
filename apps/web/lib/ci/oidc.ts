const GITHUB_OIDC_ISSUER = "https://token.actions.githubusercontent.com";
const GITHUB_OIDC_JWKS_URL = `${GITHUB_OIDC_ISSUER}/.well-known/jwks`;
const CLOCK_SKEW_SECONDS = 60;
const JWK_CACHE_TTL_MS = 5 * 60 * 1000;

type OidcJwtHeader = {
  alg?: string;
  typ?: string;
  kid?: string;
};

export type GithubOidcClaims = {
  iss: string;
  sub: string;
  aud: string | string[];
  exp: number;
  iat?: number;
  nbf?: number;
  repository?: string;
  repository_owner?: string;
  repository_id?: string;
  repository_owner_id?: string;
  ref?: string;
  ref_type?: string;
  job_workflow_ref?: string;
  workflow_ref?: string;
  event_name?: string;
  [key: string]: unknown;
};

type JsonWebKeySet = {
  keys?: GithubJsonWebKey[];
};

type GithubJsonWebKey = JsonWebKey & {
  kid?: string;
};

type CachedJwks = {
  fetchedAt: number;
  keysByKid: Map<string, GithubJsonWebKey>;
};

let cachedJwks: CachedJwks | null = null;

export async function verifyGithubOidcToken(
  token: string,
  expectedAudience: string
): Promise<GithubOidcClaims> {
  const [encodedHeader, encodedPayload, encodedSignature] = token.split(".", 3);
  if (!encodedHeader || !encodedPayload || !encodedSignature) {
    throw new Error("Malformed JWT token.");
  }

  const header = decodeSegmentJson<OidcJwtHeader>(encodedHeader, "JWT header");
  if (header.alg !== "RS256") {
    throw new Error(`Unsupported JWT algorithm '${header.alg ?? "unknown"}'.`);
  }
  if (!header.kid) {
    throw new Error("JWT header is missing 'kid'.");
  }

  const payload = decodeSegmentJson<GithubOidcClaims>(encodedPayload, "JWT payload");
  if (payload.iss !== GITHUB_OIDC_ISSUER) {
    throw new Error(`Invalid issuer '${payload.iss ?? "unknown"}'.`);
  }

  const audiences = Array.isArray(payload.aud) ? payload.aud : [payload.aud];
  if (!audiences.includes(expectedAudience)) {
    throw new Error("Invalid OIDC audience.");
  }

  const key = await loadCryptoKeyForKid(header.kid);
  const signature = ensureArrayBufferView(decodeBase64Url(encodedSignature));
  const signedContent = ensureArrayBufferView(
    new TextEncoder().encode(`${encodedHeader}.${encodedPayload}`)
  );

  const verified = await crypto.subtle.verify(
    "RSASSA-PKCS1-v1_5",
    key,
    signature,
    signedContent
  );
  if (!verified) {
    throw new Error("Invalid JWT signature.");
  }

  validateTokenTimeBounds(payload);
  return payload;
}

async function loadCryptoKeyForKid(kid: string): Promise<CryptoKey> {
  const jwk = await loadJwkByKid(kid);
  return crypto.subtle.importKey(
    "jwk",
    jwk,
    {
      name: "RSASSA-PKCS1-v1_5",
      hash: "SHA-256",
    },
    false,
    ["verify"]
  );
}

async function loadJwkByKid(kid: string): Promise<GithubJsonWebKey> {
  const now = Date.now();
  if (cachedJwks && now - cachedJwks.fetchedAt < JWK_CACHE_TTL_MS) {
    const cached = cachedJwks.keysByKid.get(kid);
    if (cached) {
      return cached;
    }
  }

  const response = await fetch(GITHUB_OIDC_JWKS_URL, { cache: "no-store" });
  if (!response.ok) {
    throw new Error("Unable to fetch GitHub OIDC signing keys.");
  }

  const body = (await response.json()) as JsonWebKeySet;
  const keysByKid = new Map<string, GithubJsonWebKey>();
  for (const key of body.keys ?? []) {
    if (key.kid) {
      keysByKid.set(key.kid, key);
    }
  }

  cachedJwks = {
    fetchedAt: now,
    keysByKid,
  };

  const selected = keysByKid.get(kid);
  if (!selected) {
    throw new Error("GitHub OIDC key ID was not found in JWKS.");
  }
  return selected;
}

function validateTokenTimeBounds(payload: GithubOidcClaims) {
  const now = Math.floor(Date.now() / 1000);
  if (typeof payload.exp !== "number") {
    throw new Error("JWT payload is missing 'exp'.");
  }

  if (now > payload.exp + CLOCK_SKEW_SECONDS) {
    throw new Error("OIDC token has expired.");
  }

  if (typeof payload.nbf === "number" && now + CLOCK_SKEW_SECONDS < payload.nbf) {
    throw new Error("OIDC token is not valid yet.");
  }

  if (typeof payload.iat === "number" && payload.iat > now + CLOCK_SKEW_SECONDS) {
    throw new Error("OIDC token has an invalid 'iat' claim.");
  }
}

function decodeSegmentJson<T>(segment: string, label: string): T {
  try {
    return JSON.parse(Buffer.from(decodeBase64Url(segment)).toString("utf8")) as T;
  } catch {
    throw new Error(`Invalid ${label}.`);
  }
}

function decodeBase64Url(value: string): Uint8Array<ArrayBuffer> {
  const padded = value.padEnd(value.length + ((4 - (value.length % 4)) % 4), "=");
  const normalized = padded.replace(/-/g, "+").replace(/_/g, "/");
  const bytes = Uint8Array.from(Buffer.from(normalized, "base64"));
  return ensureArrayBufferView(bytes);
}

function ensureArrayBufferView(value: Uint8Array): Uint8Array<ArrayBuffer> {
  const copy = new Uint8Array(value.byteLength);
  copy.set(value);
  return copy;
}
