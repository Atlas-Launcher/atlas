import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
import { createPackWithDefaults } from "@/lib/packs/create-pack";

export async function GET(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const isAdmin = hasRole(session, ["admin"]);

  const result = isAdmin
    ? await db.select().from(packs).orderBy(packs.createdAt)
    : await db
        .select({
          id: packs.id,
          name: packs.name,
          slug: packs.slug,
          description: packs.description,
          repoUrl: packs.repoUrl,
          createdAt: packs.createdAt,
          updatedAt: packs.updatedAt,
        })
        .from(packMembers)
        .innerJoin(packs, eq(packMembers.packId, packs.id))
        .where(eq(packMembers.userId, session.user.id));

  return NextResponse.json({ packs: result });
}

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json();
  const name = body?.name?.toString().trim();
  const description = body?.description?.toString().trim();
  const repoUrl = body?.repoUrl?.toString().trim();
  const slug = body?.slug?.toString().trim();

  if (!name) {
    return NextResponse.json({ error: "Name is required" }, { status: 400 });
  }

  const created = await createPackWithDefaults({
    ownerId: session.user.id,
    name,
    description,
    repoUrl,
    slug,
  });

  return NextResponse.json({ pack: created }, { status: 201 });
}
