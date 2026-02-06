import { NextResponse } from "next/server";
import crypto from "crypto";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { invites, packMembers } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

function generateCode() {
  return crypto.randomBytes(6).toString("hex");
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
    .select()
    .from(invites)
    .where(eq(invites.packId, packId))
    .orderBy(desc(invites.createdAt));

  const origin = new URL(request.url).origin;
  const serializedInvites = result.map((invite) => ({
    ...invite,
    inviteUrl: `${origin}/invite?code=${invite.code}`,
  }));

  return NextResponse.json({ invites: serializedInvites });
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

  const body = await request.json().catch(() => ({}));
  const email = body?.email?.toString().trim();
  const expiresAt = body?.expiresAt ? new Date(body.expiresAt) : null;

  const [created] = await db
    .insert(invites)
    .values({
      packId,
      role: "player",
      accessLevel: "production",
      email,
      expiresAt,
      code: generateCode(),
      createdBy: session.user.id,
    })
    .returning();

  const origin = new URL(request.url).origin;
  const serializedInvite = {
    ...created,
    inviteUrl: `${origin}/invite?code=${created.code}`,
  };

  return NextResponse.json({ invite: serializedInvite }, { status: 201 });
}

export async function DELETE(request: Request, { params }: RouteParams) {
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

  const body = await request.json().catch(() => ({}));
  const inviteId = body?.inviteId?.toString();

  if (!inviteId) {
    return NextResponse.json({ error: "inviteId is required" }, { status: 400 });
  }

  const [deleted] = await db
    .delete(invites)
    .where(and(eq(invites.id, inviteId), eq(invites.packId, packId)))
    .returning({ id: invites.id });

  if (!deleted) {
    return NextResponse.json({ error: "Invite not found" }, { status: 404 });
  }

  return NextResponse.json({ ok: true, inviteId: deleted.id });
}
