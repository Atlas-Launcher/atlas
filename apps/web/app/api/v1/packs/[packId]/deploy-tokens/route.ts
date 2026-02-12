import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { packDeployTokens, packMembers, users } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import {
  generatePackDeployToken,
  getDeployTokenPrefix,
  hashDeployToken,
} from "@/lib/auth/deploy-tokens";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

async function ensurePackTokenAccess(userId: string, packId: string) {
  const [user] = await db
    .select({ role: users.role })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1);

  if (user?.role === "admin") {
    return true;
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  return Boolean(membership && (membership.role === "creator" || membership.role === "admin"));
}

export async function GET(request: Request, { params }: RouteParams) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { packId } = await params;
  const allowed = await ensurePackTokenAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const tokens = await db
    .select({
      id: packDeployTokens.id,
      name: packDeployTokens.name,
      tokenPrefix: packDeployTokens.tokenPrefix,
      createdAt: packDeployTokens.createdAt,
      lastUsedAt: packDeployTokens.lastUsedAt,
      revokedAt: packDeployTokens.revokedAt,
      expiresAt: packDeployTokens.expiresAt,
    })
    .from(packDeployTokens)
    .where(eq(packDeployTokens.packId, packId));

  return NextResponse.json({ tokens });
}

export async function POST(request: Request, { params }: RouteParams) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { packId } = await params;
  const allowed = await ensurePackTokenAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => null);
  const name = body?.name?.toString().trim() || null;
  const expiresAtRaw = body?.expiresAt?.toString().trim();
  const expiresAt = expiresAtRaw ? new Date(expiresAtRaw) : null;
  if (expiresAt && Number.isNaN(expiresAt.getTime())) {
    return NextResponse.json({ error: "Invalid expiresAt timestamp." }, { status: 400 });
  }

  const token = generatePackDeployToken();
  const tokenHash = hashDeployToken(token);
  const tokenPrefix = getDeployTokenPrefix(token);

  const [created] = await db
    .insert(packDeployTokens)
    .values({
      packId,
      name,
      tokenHash,
      tokenPrefix,
      expiresAt: expiresAt ?? undefined,
    })
    .returning({
      id: packDeployTokens.id,
      name: packDeployTokens.name,
      tokenPrefix: packDeployTokens.tokenPrefix,
      createdAt: packDeployTokens.createdAt,
      lastUsedAt: packDeployTokens.lastUsedAt,
      revokedAt: packDeployTokens.revokedAt,
      expiresAt: packDeployTokens.expiresAt,
    });

  return NextResponse.json({ token, record: created });
}

export async function DELETE(request: Request, { params }: RouteParams) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { packId } = await params;
  const allowed = await ensurePackTokenAccess(userId, packId);
  if (!allowed) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => null);
  const tokenId = body?.tokenId?.toString().trim();
  if (!tokenId) {
    return NextResponse.json({ error: "tokenId is required." }, { status: 400 });
  }

  const now = new Date();
  await db
    .update(packDeployTokens)
    .set({ revokedAt: now })
    .where(and(eq(packDeployTokens.packId, packId), eq(packDeployTokens.id, tokenId)));

  return NextResponse.json({ revokedAt: now.toISOString() });
}
