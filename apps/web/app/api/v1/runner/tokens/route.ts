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

  const [user] = await db
    .select({ role: users.role })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1);

  const isAdmin = user?.role === "admin";
  if (!isAdmin) {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
      .limit(1);

    if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
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
