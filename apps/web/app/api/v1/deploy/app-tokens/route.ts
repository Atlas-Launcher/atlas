import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { appDeployTokens, users } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import {
  generateAppDeployToken,
  getDeployTokenPrefix,
  hashDeployToken,
} from "@/lib/auth/deploy-tokens";

async function requireAdminUserId(request: Request): Promise<string | null> {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return null;
  }

  const [user] = await db
    .select({ role: users.role })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1);

  if (user?.role !== "admin") {
    return null;
  }

  return userId;
}

export async function GET(request: Request) {
  const userId = await requireAdminUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const tokens = await db
    .select({
      id: appDeployTokens.id,
      name: appDeployTokens.name,
      tokenPrefix: appDeployTokens.tokenPrefix,
      createdAt: appDeployTokens.createdAt,
      lastUsedAt: appDeployTokens.lastUsedAt,
      revokedAt: appDeployTokens.revokedAt,
      expiresAt: appDeployTokens.expiresAt,
    })
    .from(appDeployTokens);

  return NextResponse.json({ tokens });
}

export async function POST(request: Request) {
  const userId = await requireAdminUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => null);
  const name = body?.name?.toString().trim() || null;
  const expiresAtRaw = body?.expiresAt?.toString().trim();
  const expiresAt = expiresAtRaw ? new Date(expiresAtRaw) : null;
  if (expiresAt && Number.isNaN(expiresAt.getTime())) {
    return NextResponse.json({ error: "Invalid expiresAt timestamp." }, { status: 400 });
  }

  const token = generateAppDeployToken();
  const tokenHash = hashDeployToken(token);
  const tokenPrefix = getDeployTokenPrefix(token);

  const [created] = await db
    .insert(appDeployTokens)
    .values({
      name,
      tokenHash,
      tokenPrefix,
      expiresAt: expiresAt ?? undefined,
    })
    .returning({
      id: appDeployTokens.id,
      name: appDeployTokens.name,
      tokenPrefix: appDeployTokens.tokenPrefix,
      createdAt: appDeployTokens.createdAt,
      lastUsedAt: appDeployTokens.lastUsedAt,
      revokedAt: appDeployTokens.revokedAt,
      expiresAt: appDeployTokens.expiresAt,
    });

  return NextResponse.json({
    token,
    record: created,
  });
}

export async function DELETE(request: Request) {
  const userId = await requireAdminUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => null);
  const tokenId = body?.tokenId?.toString().trim();
  if (!tokenId) {
    return NextResponse.json({ error: "tokenId is required." }, { status: 400 });
  }

  const now = new Date();
  await db
    .update(appDeployTokens)
    .set({ revokedAt: now })
    .where(eq(appDeployTokens.id, tokenId));

  return NextResponse.json({ revokedAt: now.toISOString() });
}
