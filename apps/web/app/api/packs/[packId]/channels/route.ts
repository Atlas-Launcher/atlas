import { NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { builds, channels, packMembers } from "@/lib/db/schema";
import { allowedChannels, hasRole } from "@/lib/auth/roles";

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

  const isAdmin = hasRole(session, ["admin"]);
  let accessLevel: "dev" | "beta" | "production" | "all" = "production";
  let memberRole: "admin" | "creator" | "player" = "player";

  if (!isAdmin) {
    const [membership] = await db
      .select({
        accessLevel: packMembers.accessLevel,
        role: packMembers.role,
      })
      .from(packMembers)
      .where(
        and(
          eq(packMembers.packId, packId),
          eq(packMembers.userId, session.user.id)
        )
      );

    if (!membership) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }

    accessLevel = membership.accessLevel;
    memberRole = membership.role;
  }

  const allowed = isAdmin
    ? (["dev", "beta", "production"] as const)
    : allowedChannels(accessLevel, memberRole);
  const allowedList = [...allowed];

  const result = await db
    .select({
      id: channels.id,
      name: channels.name,
      updatedAt: channels.updatedAt,
      buildId: channels.buildId,
      buildVersion: builds.version,
      buildCommit: builds.commitHash,
    })
    .from(channels)
    .leftJoin(builds, eq(builds.id, channels.buildId))
    .where(and(eq(channels.packId, packId), inArray(channels.name, allowedList)));

  return NextResponse.json({ channels: result });
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
  const channel = body?.channel?.toString();
  const buildId = body?.buildId?.toString();

  if (!channel || !buildId) {
    return NextResponse.json(
      { error: "Channel and buildId are required" },
      { status: 400 }
    );
  }

  const [updated] = await db
    .insert(channels)
    .values({
      packId,
      name: channel,
      buildId,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: [channels.packId, channels.name],
      set: { buildId, updatedAt: new Date() },
    })
    .returning();

  return NextResponse.json({ channel: updated });
}
