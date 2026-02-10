
import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";
import { db } from "@/lib/db";
import { packMembers, users } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    const runnerPackId = await getAuthenticatedRunnerPackId(request);
    if (!runnerPackId) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
    if (runnerPackId !== packId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  } else {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
      .limit(1);

    if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  // Build whitelist from pack members with Mojang info
  const members = await db
    .select({
      userId: packMembers.userId,
      role: packMembers.role,
      mojangUuid: users.mojangUuid,
      mojangUsername: users.mojangUsername,
    })
    .from(packMembers)
    .innerJoin(users, eq(packMembers.userId, users.id))
    .where(eq(packMembers.packId, packId));

  // Only include members with valid mojangUuid
  const whitelist = members
    .filter(m => m.mojangUuid)
    .map(m => ({
      name: m.mojangUsername || m.userId,
      uuid: m.mojangUuid,
      role: m.role,
    }));

  return NextResponse.json({ packId, whitelist });
}
