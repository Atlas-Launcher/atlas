import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { packMembers, packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";

interface RouteParams {
  params: Promise<{
    packId: string;
  }>;
}

async function canManagePack(request: Request, packId: string) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return { session, error: NextResponse.json({ error: "Unauthorized" }, { status: 401 }) };
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return { session, error: NextResponse.json({ error: "Forbidden" }, { status: 403 }) };
  }

  const [pack] = await db
    .select({ id: packs.id })
    .from(packs)
    .where(eq(packs.id, packId))
    .limit(1);

  if (!pack) {
    return { session, error: NextResponse.json({ error: "Pack not found" }, { status: 404 }) };
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
      )
      .limit(1);

    if (!membership || membership.role === "player") {
      return { session, error: NextResponse.json({ error: "Forbidden" }, { status: 403 }) };
    }
  }

  return { session, error: null as Response | null };
}

export async function PATCH(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const authCheck = await canManagePack(request, packId);

  if (authCheck.error) {
    return authCheck.error;
  }

  const body = await request.json().catch(() => ({}));
  const friendlyName = body?.friendlyName?.toString().trim();

  if (!friendlyName) {
    return NextResponse.json({ error: "Friendly name is required" }, { status: 400 });
  }

  const [updated] = await db
    .update(packs)
    .set({
      name: friendlyName,
      updatedAt: new Date(),
    })
    .where(eq(packs.id, packId))
    .returning({
      id: packs.id,
      name: packs.name,
      slug: packs.slug,
      updatedAt: packs.updatedAt,
    });

  return NextResponse.json({ pack: updated });
}

export async function DELETE(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const authCheck = await canManagePack(request, packId);

  if (authCheck.error) {
    return authCheck.error;
  }

  await db.delete(packs).where(eq(packs.id, packId));

  return NextResponse.json({ ok: true });
}
