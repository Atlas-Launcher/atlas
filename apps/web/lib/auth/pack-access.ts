import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { packMembers, packs } from "@/lib/db/schema";

export async function getPackMembership(userId: string, packId: string) {
  const [membership] = await db
    .select({
      packId: packMembers.packId,
      userId: packMembers.userId,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
    })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)));

  return membership;
}

export async function getPackById(packId: string) {
  const [pack] = await db.select().from(packs).where(eq(packs.id, packId));
  return pack;
}
