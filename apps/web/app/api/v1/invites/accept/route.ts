import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { invites, packMembers, packs } from "@/lib/db/schema";
import { emitWhitelistUpdate } from "@/lib/whitelist-events";
import { recomputeWhitelist } from "@/lib/packs/whitelist";

type RecommendedChannel = "dev" | "beta" | "production";

function toRecommendedChannel(accessLevel: "dev" | "beta" | "production" | "all"): RecommendedChannel {
  if (accessLevel === "dev") {
    return "dev";
  }
  if (accessLevel === "beta") {
    return "beta";
  }
  return "production";
}

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json();
  const code = body?.code?.toString().trim();

  if (!code) {
    return NextResponse.json({ error: "Invite code is required" }, { status: 400 });
  }

  const [invite] = await db
    .select()
    .from(invites)
    .where(eq(invites.code, code))
    .limit(1);

  if (!invite) {
    return NextResponse.json({ error: "Invite not found" }, { status: 404 });
  }

  if (invite.expiresAt && invite.expiresAt < new Date()) {
    return NextResponse.json({ error: "Invite expired" }, { status: 410 });
  }

  if (!invite.packId) {
    return NextResponse.json({ error: "Invite missing pack" }, { status: 400 });
  }

  const [pack] = await db
    .select({
      id: packs.id,
      name: packs.name,
      slug: packs.slug,
    })
    .from(packs)
    .where(eq(packs.id, invite.packId))
    .limit(1);

  if (!pack) {
    return NextResponse.json({ error: "Invite pack not found" }, { status: 404 });
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

  await recomputeWhitelist(invite.packId);

  emitWhitelistUpdate({ packId: invite.packId, source: "invite" });

  const recommendedChannel = toRecommendedChannel(invite.accessLevel);

  return NextResponse.json({
    success: true,
    packId: invite.packId,
    pack,
    recommendedChannel,
  });
}
