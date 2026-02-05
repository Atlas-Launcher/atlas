import { NextResponse } from "next/server";
import { and, asc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { packMembers, users } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";

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
  if (!isAdmin) {
    const [membership] = await db
      .select({ userId: packMembers.userId })
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
  }

  const result = await db
    .select({
      userId: users.id,
      name: users.name,
      email: users.email,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
      joinedAt: packMembers.createdAt,
    })
    .from(packMembers)
    .innerJoin(users, eq(packMembers.userId, users.id))
    .where(eq(packMembers.packId, packId))
    .orderBy(asc(users.name));

  return NextResponse.json({ members: result });
}
