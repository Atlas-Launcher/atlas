import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { invites, packs, users } from "@/lib/db/schema";

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const code = searchParams.get("code")?.trim();

  if (!code) {
    return NextResponse.json({ error: "Invite code is required" }, { status: 400 });
  }

  const [invite] = await db
    .select({
      code: invites.code,
      packId: invites.packId,
      packName: packs.name,
      packOwnerId: packs.ownerId,
      creatorName: users.name,
      creatorEmail: users.email,
      expiresAt: invites.expiresAt,
      createdBy: invites.createdBy,
    })
    .from(invites)
    .leftJoin(packs, eq(invites.packId, packs.id))
    .leftJoin(users, eq(packs.ownerId, users.id))
    .where(eq(invites.code, code));

  if (!invite) {
    return NextResponse.json({ error: "Invite not found" }, { status: 404 });
  }

  if (invite.expiresAt && invite.expiresAt < new Date()) {
    return NextResponse.json({ error: "Invite expired" }, { status: 410 });
  }

  if (!invite.packId || !invite.packName) {
    return NextResponse.json({ error: "Invite missing pack" }, { status: 400 });
  }

  let creatorName = invite.creatorName ?? null;
  let creatorEmail = invite.creatorEmail ?? null;

  if (!creatorName && invite.createdBy) {
    const [creator] = await db
      .select({ name: users.name, email: users.email })
      .from(users)
      .where(eq(users.id, invite.createdBy));
    creatorName = creator?.name ?? null;
    creatorEmail = creator?.email ?? null;
  }

  return NextResponse.json({
    pack: {
      id: invite.packId,
      name: invite.packName,
    },
    creator: {
      name: creatorName,
      email: creatorEmail,
    },
  });
}
