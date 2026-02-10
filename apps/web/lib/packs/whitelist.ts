import { eq, and, isNotNull, sql } from "drizzle-orm";
import { db } from "@/lib/db";
import { packs, packMembers, users, packWhitelists } from "@/lib/db/schema";

export async function incrementWhitelistVersion(packId: string) {
  await db
    .update(packs)
    .set({
      whitelistVersion: sql`${packs.whitelistVersion} + 1`,
    })
    .where(eq(packs.id, packId));
}

export async function recomputeWhitelist(packId: string) {
  // Get current members with Mojang info
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

  const json = JSON.stringify(whitelist);

  // Get current version
  const [pack] = await db
    .select({ version: packs.whitelistVersion })
    .from(packs)
    .where(eq(packs.id, packId));

  const currentVersion = pack?.version ?? 0;

  // Upsert the precomputed whitelist
  await db
    .insert(packWhitelists)
    .values({
      packId,
      version: currentVersion,
      json,
    })
    .onConflictDoUpdate({
      target: packWhitelists.packId,
      set: {
        version: currentVersion,
        json,
        updatedAt: sql`now()`,
      },
    });

  // Increment version for next time
  await incrementWhitelistVersion(packId);
}

export async function getWhitelistVersion(packId: string) {
  const [row] = await db
    .select({ version: packWhitelists.version })
    .from(packWhitelists)
    .where(eq(packWhitelists.packId, packId))
    .limit(1);

  if (!row) {
    await recomputeWhitelist(packId);
    const [row2] = await db
      .select({ version: packWhitelists.version })
      .from(packWhitelists)
      .where(eq(packWhitelists.packId, packId))
      .limit(1);

    if (!row2) throw new Error(`Whitelist missing after recompute for packId=${packId}`);
    return row2.version;
  }

  return row.version;
}

export async function getWhitelistByVersion(
  packId: string,
  version: number,
  opts?: { recomputeIfMissing?: boolean }
): Promise<{ version: number; data: unknown } | null> {
  const [row] = await db
    .select({
      version: packWhitelists.version,
      json: packWhitelists.json,
    })
    .from(packWhitelists)
    .where(and(eq(packWhitelists.packId, packId), eq(packWhitelists.version, version)))
    .limit(1);

  if (row) {
    return { version: row.version, data: JSON.parse(row.json) as unknown };
  }

  if (!opts?.recomputeIfMissing) {
    return null;
  }

  // If there was no row at that version, it might be because the whitelist row doesn't exist yet.
  // Recompute once, then try again.
  await recomputeWhitelist(packId);

  const [row2] = await db
    .select({
      version: packWhitelists.version,
      json: packWhitelists.json,
    })
    .from(packWhitelists)
    .where(and(eq(packWhitelists.packId, packId), eq(packWhitelists.version, version)))
    .limit(1);

  if (!row2) return null;

  return { version: row2.version, data: JSON.parse(row2.json) as unknown };
}

export async function getWhitelist(packId: string) {
  const [row] = await db
    .select({
      version: packWhitelists.version,
      json: packWhitelists.json,
    })
    .from(packWhitelists)
    .where(eq(packWhitelists.packId, packId))
    .limit(1);

  if (!row) {
    await recomputeWhitelist(packId);

    const [row2] = await db
      .select({
        version: packWhitelists.version,
        json: packWhitelists.json,
      })
      .from(packWhitelists)
      .where(eq(packWhitelists.packId, packId))
      .limit(1);

    if (!row2) {
      throw new Error(`Whitelist missing after recompute for packId=${packId}`);
    }

    return { version: row2.version, data: JSON.parse(row2.json) };
  }

  return { version: row.version, data: JSON.parse(row.json) };
}
