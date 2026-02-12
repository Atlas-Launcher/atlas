import { createHash, randomBytes, timingSafeEqual } from "crypto";
import { and, eq, gt, isNull, or } from "drizzle-orm";

import { db } from "@/lib/db";
import { appDeployTokens, packDeployTokens } from "@/lib/db/schema";

const APP_TOKEN_PREFIX = "atlas_app_";
const PACK_TOKEN_PREFIX = "atlas_pack_";
const TOKEN_PREFIX_LENGTH = 20;

type DeployTokenRecord = {
  id: string;
  packId?: string;
};

function hashToken(token: string): string {
  return createHash("sha256").update(token).digest("base64");
}

function getTokenPrefix(token: string): string {
  return token.slice(0, TOKEN_PREFIX_LENGTH);
}

function createToken(prefix: string): string {
  return `${prefix}${randomBytes(32).toString("base64url")}`;
}

function tokensEqual(left: string, right: string): boolean {
  const leftBuf = Buffer.from(left, "base64");
  const rightBuf = Buffer.from(right, "base64");
  if (leftBuf.length !== rightBuf.length) {
    return false;
  }
  return timingSafeEqual(leftBuf, rightBuf);
}

export function generateAppDeployToken() {
  return createToken(APP_TOKEN_PREFIX);
}

export function generatePackDeployToken() {
  return createToken(PACK_TOKEN_PREFIX);
}

export function getDeployTokenPrefix(token: string) {
  return getTokenPrefix(token);
}

export function hashDeployToken(token: string) {
  return hashToken(token);
}

export async function resolveAppDeployToken(token: string): Promise<DeployTokenRecord | null> {
  const prefix = getTokenPrefix(token);
  const hashed = hashToken(token);
  const now = new Date();

  const candidates = await db
    .select({
      id: appDeployTokens.id,
      tokenHash: appDeployTokens.tokenHash,
    })
    .from(appDeployTokens)
    .where(
      and(
        eq(appDeployTokens.tokenPrefix, prefix),
        isNull(appDeployTokens.revokedAt),
        or(isNull(appDeployTokens.expiresAt), gt(appDeployTokens.expiresAt, now))
      )
    );

  for (const candidate of candidates) {
    if (tokensEqual(candidate.tokenHash, hashed)) {
      return { id: candidate.id };
    }
  }

  return null;
}

export async function resolvePackDeployToken(token: string): Promise<DeployTokenRecord | null> {
  const prefix = getTokenPrefix(token);
  const hashed = hashToken(token);
  const now = new Date();

  const candidates = await db
    .select({
      id: packDeployTokens.id,
      packId: packDeployTokens.packId,
      tokenHash: packDeployTokens.tokenHash,
    })
    .from(packDeployTokens)
    .where(
      and(
        eq(packDeployTokens.tokenPrefix, prefix),
        isNull(packDeployTokens.revokedAt),
        or(isNull(packDeployTokens.expiresAt), gt(packDeployTokens.expiresAt, now))
      )
    );

  for (const candidate of candidates) {
    if (tokensEqual(candidate.tokenHash, hashed)) {
      return { id: candidate.id, packId: candidate.packId };
    }
  }

  return null;
}

export async function touchAppDeployToken(id: string) {
  await db
    .update(appDeployTokens)
    .set({ lastUsedAt: new Date() })
    .where(eq(appDeployTokens.id, id));
}

export async function touchPackDeployToken(id: string) {
  await db
    .update(packDeployTokens)
    .set({ lastUsedAt: new Date() })
    .where(eq(packDeployTokens.id, id));
}

export async function rotateManagedPackDeployToken(packId: string, name: string) {
  const token = generatePackDeployToken();
  const tokenHash = hashDeployToken(token);
  const tokenPrefix = getDeployTokenPrefix(token);
  const now = new Date();

  await db
    .update(packDeployTokens)
    .set({ revokedAt: now })
    .where(
      and(
        eq(packDeployTokens.packId, packId),
        eq(packDeployTokens.name, name),
        isNull(packDeployTokens.revokedAt)
      )
    );

  await db.insert(packDeployTokens).values({
    packId,
    name,
    tokenHash,
    tokenPrefix,
  });

  return token;
}
