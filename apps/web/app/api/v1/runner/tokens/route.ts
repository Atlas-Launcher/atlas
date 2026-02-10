import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { packMembers, runnerServiceTokens, users } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import {
  generateServiceToken,
  getServiceTokenPrefix,
  hashServiceToken,
} from "@/lib/auth/runner-tokens";

export const runtime = "nodejs";

async function ensurePackAccess(userId: string, packId: string) {
  const [user] = await db
    .select({ role: users.role })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1);

  const isAdmin = user?.role === "admin";
  if (isAdmin) {
    return true;
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  return Boolean(membership && (membership.role === "creator" || membership.role === "admin"));
}

export async function GET(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { searchParams } = new URL(request.url);
  const packId = searchParams.get("packId")?.trim();
  if (!packId) {
    return NextResponse.json({ error: "packId is required." }, { status: 400 });
  }

  const allowed = await ensurePackAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const tokens = await db
    .select({
      id: runnerServiceTokens.id,
      name: runnerServiceTokens.name,
      tokenPrefix: runnerServiceTokens.tokenPrefix,
      createdAt: runnerServiceTokens.createdAt,
      lastUsedAt: runnerServiceTokens.lastUsedAt,
      revokedAt: runnerServiceTokens.revokedAt,
      expiresAt: runnerServiceTokens.expiresAt,
    })
    .from(runnerServiceTokens)
    .where(eq(runnerServiceTokens.packId, packId))
    .orderBy(runnerServiceTokens.createdAt);

  return NextResponse.json({ tokens });
}

export async function POST(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => null);
  const packId = body?.packId?.toString().trim();
  const name = body?.name?.toString().trim();
  if (!packId) {
    return NextResponse.json({ error: "packId is required." }, { status: 400 });
  }

  const allowed = await ensurePackAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const token = generateServiceToken();
  const tokenHash = hashServiceToken(token);
  const tokenPrefix = getServiceTokenPrefix(token);

  const [created] = await db
    .insert(runnerServiceTokens)
    .values({
      packId,
      name: name || null,
      tokenHash,
      tokenPrefix,
    })
    .returning({ id: runnerServiceTokens.id });

  return NextResponse.json({
    id: created?.id ?? null,
    packId,
    token,
    prefix: tokenPrefix,
  });
}

export async function DELETE(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => null);
  const packId = body?.packId?.toString().trim();
  const tokenId = body?.tokenId?.toString().trim();
  if (!packId || !tokenId) {
    return NextResponse.json({ error: "packId and tokenId are required." }, { status: 400 });
  }

  const allowed = await ensurePackAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const now = new Date();
  await db
    .update(runnerServiceTokens)
    .set({ revokedAt: now })
    .where(and(eq(runnerServiceTokens.packId, packId), eq(runnerServiceTokens.id, tokenId)));

  return NextResponse.json({ revokedAt: now.toISOString() });
}
