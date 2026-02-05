import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { deployTokens, packMembers } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
import { generateDeployToken, hashDeployToken } from "@/lib/auth/deploy-tokens";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(
      and(
        eq(packMembers.packId, packId),
        eq(packMembers.userId, session.user.id)
      )
    );

  if (!membership && !hasRole(session, ["admin"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const result = await db
    .select({
      id: deployTokens.id,
      label: deployTokens.label,
      active: deployTokens.active,
      createdAt: deployTokens.createdAt,
      revokedAt: deployTokens.revokedAt,
    })
    .from(deployTokens)
    .where(eq(deployTokens.packId, packId))
    .orderBy(desc(deployTokens.createdAt));

  return NextResponse.json({ tokens: result });
}

export async function POST(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(
      and(
        eq(packMembers.packId, packId),
        eq(packMembers.userId, session.user.id)
      )
    );

  if (!membership && !hasRole(session, ["admin"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json();
  const label = body?.label?.toString().trim();

  const token = generateDeployToken();
  const tokenHash = hashDeployToken(token);

  const [created] = await db
    .insert(deployTokens)
    .values({
      packId,
      tokenHash,
      label,
      active: true,
    })
    .returning({
      id: deployTokens.id,
      label: deployTokens.label,
      active: deployTokens.active,
      createdAt: deployTokens.createdAt,
    });

  return NextResponse.json({ token, record: created }, { status: 201 });
}
