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

  const rows = await db
    .select()
    .from(builds)
    .where(eq(builds.packId, packId))
    .orderBy(desc(builds.createdAt));

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
