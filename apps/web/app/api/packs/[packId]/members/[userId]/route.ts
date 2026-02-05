import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { packMembers } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";

interface RouteParams {
  params: Promise<{
    packId: string;
    userId: string;
  }>;
}

export async function DELETE(request: Request, { params }: RouteParams) {
  const { packId, userId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  if (!hasRole(session, ["admin"])) {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(
        and(
          eq(packMembers.packId, packId),
          eq(packMembers.userId, session.user.id)
        )
      );

    if (!membership || membership.role === "player") {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  await db
    .delete(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)));

  return NextResponse.json({ ok: true });
}
