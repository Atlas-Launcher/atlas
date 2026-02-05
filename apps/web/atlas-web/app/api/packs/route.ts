import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { channels, packMembers, packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-");
}

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
  const slug = body?.slug?.toString().trim() || (name ? slugify(name) : "");

  if (!name || !slug) {
    return NextResponse.json({ error: "Name and slug are required" }, { status: 400 });
  }

  const [created] = await db
    .insert(packs)
    .values({
      name,
      slug,
      description,
      repoUrl,
      ownerId: session.user.id,
    })
    .returning();

  await db.insert(packMembers).values({
    packId: created.id,
    userId: session.user.id,
    role: "creator",
    accessLevel: "dev",
  });

  await db.insert(channels).values([
    { packId: created.id, name: "dev" },
    { packId: created.id, name: "beta" },
    { packId: created.id, name: "production" },
  ]);

  return NextResponse.json({ pack: created }, { status: 201 });
}
