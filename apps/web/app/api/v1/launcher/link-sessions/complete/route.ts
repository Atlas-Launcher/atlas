import { NextResponse } from "next/server";
import { and, eq, ne } from "drizzle-orm";

import { db } from "@/lib/db";
import { launcherLinkSessions, users, packMembers } from "@/lib/db/schema";
import { canonicalizeMinecraftUuid } from "@/lib/minecraft/uuid";
import { recomputeWhitelist } from "@/lib/packs/whitelist";

export async function POST(request: Request) {
  const body = await request.json().catch(() => ({}));
  const linkSessionId = body?.linkSessionId?.toString().trim();
  const proof = body?.proof?.toString().trim();
  const minecraftUuid = canonicalizeMinecraftUuid(body?.minecraft?.uuid?.toString());
  const minecraftName = body?.minecraft?.name?.toString().trim();

  if (!linkSessionId || !proof) {
    return NextResponse.json({ error: "linkSessionId and proof are required" }, { status: 400 });
  }

  if (!minecraftUuid || !minecraftName) {
    return NextResponse.json(
      { error: "Valid Minecraft identity is required" },
      { status: 400 }
    );
  }

  const [linkSession] = await db
    .select()
    .from(launcherLinkSessions)
    .where(eq(launcherLinkSessions.id, linkSessionId));

  if (!linkSession) {
    return NextResponse.json({ error: "Link session not found" }, { status: 404 });
  }

  if (linkSession.expiresAt < new Date()) {
    return NextResponse.json({ error: "Link session expired" }, { status: 410 });
  }

  if (linkSession.completedAt) {
    return NextResponse.json({ error: "Link session already completed" }, { status: 409 });
  }

  if (!linkSession.claimedUserId) {
    return NextResponse.json({ error: "Link session not claimed" }, { status: 409 });
  }

  if (linkSession.proof !== proof) {
    return NextResponse.json({ error: "Invalid proof" }, { status: 401 });
  }

  const [existingUser] = await db
    .select({ id: users.id })
    .from(users)
    .where(and(eq(users.mojangUuid, minecraftUuid), ne(users.id, linkSession.claimedUserId)));

  if (existingUser) {
    return NextResponse.json(
      { error: "Minecraft account already linked to another user" },
      { status: 409 }
    );
  }

  // Allow replacing the user's existing linked Minecraft account.
  if (linkSession.claimedUserId) {
    await db
      .update(users)
      .set({
        mojangUuid: null,
        mojangUsername: null,
        updatedAt: new Date(),
      })
      .where(eq(users.id, linkSession.claimedUserId));

    // Increment whitelist versions when clearing Mojang info
    const userPacks = await db
      .select({ packId: packMembers.packId })
      .from(packMembers)
      .where(eq(packMembers.userId, linkSession.claimedUserId));

    for (const { packId } of userPacks) {
      await recomputeWhitelist(packId);
    }
  }

  await db
    .update(users)
    .set({
      mojangUuid: minecraftUuid,
      mojangUsername: minecraftName,
      updatedAt: new Date(),
    })
    .where(eq(users.id, linkSession.claimedUserId));

  // Increment whitelist versions for all packs this user is a member of
  const userPacks = await db
    .select({ packId: packMembers.packId })
    .from(packMembers)
    .where(eq(packMembers.userId, linkSession.claimedUserId));

  for (const { packId } of userPacks) {
    await recomputeWhitelist(packId);
  }

  await db
    .update(launcherLinkSessions)
    .set({
      minecraftUuid,
      minecraftName,
      completedAt: new Date(),
    })
    .where(eq(launcherLinkSessions.id, linkSession.id));

  return NextResponse.json({
    success: true,
    userId: linkSession.claimedUserId,
  });
}
