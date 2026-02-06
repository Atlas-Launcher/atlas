import { NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { builds, channels, packMembers } from "@/lib/db/schema";
import { allowedChannels, hasRole } from "@/lib/auth/roles";
import { decodeArtifactRef, isStorageProviderEnabled } from "@/lib/storage/harness";

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
      artifactKey: builds.artifactKey,
    })
    .from(channels)
    .leftJoin(builds, eq(builds.id, channels.buildId))
    .where(and(eq(channels.packId, packId), inArray(channels.name, allowedList)));

  const filtered = result.map((row) => {
    if (!row.artifactKey) {
      return {
        id: row.id,
        name: row.name,
        updatedAt: row.updatedAt,
        buildId: row.buildId,
        buildVersion: row.buildVersion,
        buildCommit: row.buildCommit,
      };
    }

    const artifactRef = decodeArtifactRef(row.artifactKey);
    if (!isStorageProviderEnabled(artifactRef.provider)) {
      return {
        id: row.id,
        name: row.name,
        updatedAt: row.updatedAt,
        buildId: null,
        buildVersion: null,
        buildCommit: null,
      };
    }

    return {
      id: row.id,
      name: row.name,
      updatedAt: row.updatedAt,
      buildId: row.buildId,
      buildVersion: row.buildVersion,
      buildCommit: row.buildCommit,
    };
  });

  return NextResponse.json({ channels: filtered });
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

  const [build] = await db
    .select({
      id: builds.id,
      artifactKey: builds.artifactKey,
    })
    .from(builds)
    .where(and(eq(builds.id, buildId), eq(builds.packId, packId)))
    .limit(1);

  if (!build) {
    return NextResponse.json({ error: "Build not found" }, { status: 404 });
  }

  const artifactRef = decodeArtifactRef(build.artifactKey);
  if (!isStorageProviderEnabled(artifactRef.provider)) {
    return NextResponse.json(
      {
        error: `Storage provider '${artifactRef.provider}' is not enabled for this build.`,
      },
      { status: 503 }
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
