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
  const [selfMembership] = await db
    .select({
      userId: packMembers.userId,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
      joinedAt: packMembers.createdAt,
    })
    .from(packMembers)
    .where(
      and(
        eq(packMembers.packId, packId),
        eq(packMembers.userId, session.user.id)
      )
    );

  if (!isAdmin && !selfMembership) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const members = await db
    .select({
      userId: users.id,
      name: users.name,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
      joinedAt: packMembers.createdAt,
    })
    .from(packMembers)
    .innerJoin(users, eq(packMembers.userId, users.id))
    .where(eq(packMembers.packId, packId))
    .orderBy(asc(users.name))
    .limit(500); // Reasonable limit for pack members

  const includesCurrentUser = members.some((member) => member.userId === session.user.id);
  if (!includesCurrentUser) {
    const fallbackRole =
      session.user.role === "admin" ||
      session.user.role === "creator" ||
      session.user.role === "player"
        ? session.user.role
        : "player";
    const fallbackAccess = fallbackRole === "player" ? "production" : "all";

    members.push({
      userId: session.user.id,
      name: session.user.name ?? "You",
      role: selfMembership?.role ?? fallbackRole,
      accessLevel: selfMembership?.accessLevel ?? fallbackAccess,
      joinedAt: selfMembership?.joinedAt ?? new Date(),
    });
  }

  return NextResponse.json({ members });
}
