import { createHash, randomBytes, timingSafeEqual } from "crypto";
import { SignJWT, jwtVerify } from "jose";
import { and, eq, isNull, or, gt } from "drizzle-orm";

import { db } from "@/lib/db";
import { runnerServiceTokens } from "@/lib/db/schema";

const SERVICE_TOKEN_PREFIX = "atlas_runner_";
const SERVICE_TOKEN_PREFIX_LENGTH = 16;
const RUNNER_TOKEN_AUDIENCE = "atlas-runner";
const RUNNER_TOKEN_ISSUER = "atlas-hub";
const DEFAULT_RUNNER_TTL_SECONDS = 60 * 60;

export type RunnerServiceTokenRecord = {
  id: string;
  packId: string;
  expiresAt: Date | null;
  revokedAt: Date | null;
};

function getRunnerTokenSecret(): Uint8Array {
  const value =
    process.env.ATLAS_RUNNER_TOKEN_SECRET ??
    process.env.BETTER_AUTH_SECRET ??
    process.env.AUTH_SECRET ??
    "";
  if (!value) {
    throw new Error("Missing ATLAS_RUNNER_TOKEN_SECRET (or BETTER_AUTH_SECRET).");
  }
  return new TextEncoder().encode(value);
}

export function getRunnerTokenTtlSeconds(): number {
  const raw = process.env.ATLAS_RUNNER_TOKEN_TTL_SECONDS?.trim();
  if (!raw) {
    return DEFAULT_RUNNER_TTL_SECONDS;
  }
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) {
    return DEFAULT_RUNNER_TTL_SECONDS;
  }
  return Math.floor(value);
}

export function generateServiceToken(): string {
  const random = randomBytes(32).toString("base64url");
  return `${SERVICE_TOKEN_PREFIX}${random}`;
}

export function getServiceTokenPrefix(token: string): string {
  return token.slice(0, SERVICE_TOKEN_PREFIX_LENGTH);
}

export function hashServiceToken(token: string): string {
  return createHash("sha256").update(token).digest("base64");
}

export async function resolveRunnerServiceToken(
  token: string
): Promise<RunnerServiceTokenRecord | null> {
  const prefix = getServiceTokenPrefix(token);
  const hashed = hashServiceToken(token);
  const now = new Date();

  const candidates = await db
    .select({
      id: runnerServiceTokens.id,
      packId: runnerServiceTokens.packId,
      tokenHash: runnerServiceTokens.tokenHash,
      revokedAt: runnerServiceTokens.revokedAt,
      expiresAt: runnerServiceTokens.expiresAt,
    })
    .from(runnerServiceTokens)
    .where(
      and(
        eq(runnerServiceTokens.tokenPrefix, prefix),
        isNull(runnerServiceTokens.revokedAt),
        or(
          isNull(runnerServiceTokens.expiresAt),
          gt(runnerServiceTokens.expiresAt, now)
        )
      )
    );

  for (const candidate of candidates) {
    const left = Buffer.from(candidate.tokenHash, "base64");
    const right = Buffer.from(hashed, "base64");
    if (left.length !== right.length) {
      continue;
    }
    if (timingSafeEqual(left, right)) {
      return {
        id: candidate.id,
        packId: candidate.packId,
        expiresAt: candidate.expiresAt,
        revokedAt: candidate.revokedAt,
      };
    }
  }

  return null;
}

export async function createRunnerAccessToken(input: {
  packId: string;
  tokenId: string;
  expiresIn?: number;
}): Promise<{ token: string; expiresIn: number }> {
  const expiresIn = input.expiresIn ?? getRunnerTokenTtlSeconds();
  const now = Math.floor(Date.now() / 1000);

  const token = await new SignJWT({
    typ: "runner",
    tid: input.tokenId,
  })
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt(now)
    .setIssuer(RUNNER_TOKEN_ISSUER)
    .setAudience(RUNNER_TOKEN_AUDIENCE)
    .setSubject(input.packId)
    .setExpirationTime(now + expiresIn)
    .sign(getRunnerTokenSecret());

  return { token, expiresIn };
}

function parseBearer(headers: Headers): string | null {
  const header = headers.get("authorization")?.trim();
  if (!header) {
    return null;
  }
  const [scheme, token] = header.split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }
  return token.trim() || null;
}

export async function getAuthenticatedRunnerPackId(
  request: Request
): Promise<string | null> {
  const bearer = parseBearer(request.headers);
  if (!bearer) {
    return null;
  }
  try {
    const { payload } = await jwtVerify(bearer, getRunnerTokenSecret(), {
      issuer: RUNNER_TOKEN_ISSUER,
      audience: RUNNER_TOKEN_AUDIENCE,
    });

    if (payload.typ !== "runner") {
      return null;
    }

    if (typeof payload.sub !== "string" || !payload.sub) {
      return null;
    }

    return payload.sub;
  } catch {
    return null;
  }
}
