import { and, eq } from "drizzle-orm";
import { nanoid } from "nanoid";

import { db } from "@/lib/db";
import { channels, packMembers, packs } from "@/lib/db/schema";

export function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-");
}

async function resolveSlug(preferred: string) {
  const base = slugify(preferred) || "atlas-pack";
  let suffix = 0;

  for (;;) {
    const candidate = suffix === 0 ? base : `${base}-${suffix}`;
    const [existing] = await db
      .select({ id: packs.id })
      .from(packs)
      .where(eq(packs.slug, candidate))
      .limit(1);

    if (!existing) {
      return candidate;
    }

    suffix += 1;
  }
}

export async function createPackWithDefaults({
  ownerId,
  name,
  description,
  repoUrl,
  slug,
}: {
  ownerId: string;
  name: string;
  description?: string;
  repoUrl?: string;
  slug?: string;
}) {
  const resolvedSlug = await resolveSlug(slug ?? name);

  const createdPack = await db.transaction(async (tx) => {
    const [created] = await tx
      .insert(packs)
      .values({
        id: nanoid(),
        name,
        slug: resolvedSlug,
        description,
        repoUrl,
        ownerId,
      })
      .returning();

    await tx.insert(packMembers).values({
      packId: created.id,
      userId: ownerId,
      role: "creator",
      accessLevel: "dev",
    });

    await tx.insert(channels).values([
      { packId: created.id, name: "dev" },
      { packId: created.id, name: "beta" },
      { packId: created.id, name: "production" },
    ]);

    return created;
  });

  return createdPack;
}

export async function canManagePackByMembership({
  packId,
  userId,
  isAdmin,
}: {
  packId: string;
  userId: string;
  isAdmin: boolean;
}) {
  if (isAdmin) {
    return true;
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  return Boolean(membership && membership.role !== "player");
}
