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

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const isAdmin = hasRole(session, ["admin"]);
  if (!isAdmin) {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, session.user.id)))
      .limit(1);

    if (!membership) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  const [pack] = await db
    .select({
      id: packs.id,
      name: packs.name,
      slug: packs.slug,
      description: packs.description,
      repoUrl: packs.repoUrl,
      ownerId: packs.ownerId,
      createdAt: packs.createdAt,
      updatedAt: packs.updatedAt,
    })
    .from(packs)
    .where(eq(packs.id, packId))
    .limit(1);

  if (!pack) {
    return NextResponse.json({ error: "Pack not found" }, { status: 404 });
  }

  return NextResponse.json({ pack });
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

  if (!hasRole(session, ["admin"])) {
    const [membership] = await db
      .select({ role: packMembers.role })
      .from(packMembers)
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, session.user.id)))
      .limit(1);

    if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  const body = await request.json().catch(() => ({}));
  const friendlyName = body?.friendlyName?.toString().trim();

  if (!friendlyName) {
    return NextResponse.json({ error: "Friendly name is required." }, { status: 400 });
  }

  const [pack] = await db
    .update(packs)
    .set({ name: friendlyName, updatedAt: new Date() })
    .where(eq(packs.id, packId))
    .returning({ id: packs.id, name: packs.name, updatedAt: packs.updatedAt });

  if (!pack) {
    return NextResponse.json({ error: "Pack not found" }, { status: 404 });
  }

  return NextResponse.json({ pack });
}

export async function DELETE(request: Request, { params }: RouteParams) {
  const { packId } = await params;
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
      .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, session.user.id)))
      .limit(1);

    if (!membership || (membership.role !== "creator" && membership.role !== "admin")) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  const [deletedPack] = await db
    .delete(packs)
    .where(eq(packs.id, packId))
    .returning({ id: packs.id });

  if (!deletedPack) {
    return NextResponse.json({ error: "Pack not found" }, { status: 404 });
  }

  return NextResponse.json({ success: true });
}
