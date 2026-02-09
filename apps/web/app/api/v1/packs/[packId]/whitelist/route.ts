import { NextResponse } from "next/server";
import { and, eq, isNotNull } from "drizzle-orm";

import { db } from "@/lib/db";
import { packMembers, users } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const userId = await getAuthenticatedUserId(request);

  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const members = await db
    .select({
      uuid: users.mojangUuid,
      name: users.mojangUsername,
    })
    .from(packMembers)
    .innerJoin(users, eq(packMembers.userId, users.id))
    .where(and(eq(packMembers.packId, packId), isNotNull(users.mojangUuid)));

  const whitelist = members
    .filter((member) => member.uuid)
    .map((member) => ({
      uuid: member.uuid as string,
      name: member.name ?? "",
    }));

  return NextResponse.json(whitelist);
}