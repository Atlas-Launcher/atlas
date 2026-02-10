import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { builds, packMembers } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
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
  const membership = isAdmin
    ? true
    : await db
        .select({ userId: packMembers.userId })
        .from(packMembers)
        .where(
          and(
            eq(packMembers.packId, packId),
            eq(packMembers.userId, session.user.id)
          )
        )
        .then((rows) => rows.length > 0);

  if (!membership) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const url = new URL(request.url);
  const limit = Math.min(parseInt(url.searchParams.get("limit") || "50"), 100); // Default 50, max 100

  const rows = await db
    .select()
    .from(builds)
    .where(eq(builds.packId, packId))
    .orderBy(desc(builds.createdAt))
    .limit(limit);

  const result = rows
    .map((build) => {
      const artifactRef = decodeArtifactRef(build.artifactKey);
      if (!isStorageProviderEnabled(artifactRef.provider)) {
        return null;
      }
      return {
        ...build,
        artifactKey: artifactRef.key,
        artifactProvider: artifactRef.provider,
      };
    })
    .filter((build): build is NonNullable<typeof build> => Boolean(build));

  return NextResponse.json({ builds: result });
}

export async function PATCH(request: Request, { params }: RouteParams) {
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
    )
    .limit(1);
  if (!membership && !hasRole(session, ["admin"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => ({}));
  const buildId = body?.buildId?.toString()?.trim();
  const forceReinstall = body?.forceReinstall;
  if (!buildId || typeof forceReinstall !== "boolean") {
    return NextResponse.json(
      { error: "buildId and forceReinstall are required." },
      { status: 400 }
    );
  }

  const [build] = await db
    .update(builds)
    .set({ forceReinstall })
    .where(and(eq(builds.id, buildId), eq(builds.packId, packId)))
    .returning();

  if (!build) {
    return NextResponse.json({ error: "Build not found." }, { status: 404 });
  }

  return NextResponse.json({ build });
}
