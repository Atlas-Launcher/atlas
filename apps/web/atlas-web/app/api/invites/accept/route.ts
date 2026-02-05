import { NextResponse } from "next/server";
import { and, desc, eq, isNull } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { invites, packMembers } from "@/lib/db/schema";

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json();
  const code = body?.code?.toString().trim();
  const packId = body?.packId?.toString().trim();

  if (!code && !packId) {
    return NextResponse.json({ error: "Invite link required" }, { status: 400 });
  }

  const [invite] = await db
    .select()
    .from(invites)
    .where(
      code
        ? and(eq(invites.code, code), isNull(invites.usedAt))
        : and(eq(invites.packId, packId!), isNull(invites.usedAt))
    )
    .orderBy(desc(invites.createdAt));

  if (!invite) {
    return NextResponse.json({ error: "Invite not found" }, { status: 404 });
  }

  if (invite.expiresAt && invite.expiresAt < new Date()) {
    return NextResponse.json({ error: "Invite expired" }, { status: 410 });
  }

  if (!invite.packId) {
    return NextResponse.json({ error: "Invite missing pack" }, { status: 400 });
  }

  await db
    .insert(packMembers)
    .values({
      packId: invite.packId,
      userId: session.user.id,
      role: invite.role,
      accessLevel: invite.accessLevel,
    })
    .onConflictDoNothing();

  if (code) {
    await db
      .update(invites)
      .set({ usedAt: new Date() })
      .where(eq(invites.id, invite.id));
  }

  return NextResponse.json({ success: true });
}
