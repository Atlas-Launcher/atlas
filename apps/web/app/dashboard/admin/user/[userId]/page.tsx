import { headers } from "next/headers";
import { notFound, redirect } from "next/navigation";
import { eq, asc } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { packMembers, packs, users } from "@/lib/db/schema";
import AdminUserClient from "@/app/dashboard/admin/user/[userId]/user-client";

interface PageProps {
  params: Promise<{
    userId: string;
  }>;
}

export default async function AdminUserPage({ params }: PageProps) {
  const session = await auth.api.getSession({ headers: await headers() });

  if (!session?.user) {
    redirect("/sign-in");
  }

  if (session.user.role !== "admin") {
    redirect("/dashboard");
  }

  const { userId } = await params;
  const [user] = await db
    .select({
      id: users.id,
      name: users.name,
      email: users.email,
      role: users.role,
      createdAt: users.createdAt,
    })
    .from(users)
    .where(eq(users.id, userId));

  if (!user) {
    notFound();
  }

  const memberships = await db
    .select({
      packId: packs.id,
      packName: packs.name,
      packSlug: packs.slug,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
      joinedAt: packMembers.createdAt,
    })
    .from(packMembers)
    .innerJoin(packs, eq(packMembers.packId, packs.id))
    .where(eq(packMembers.userId, userId))
    .orderBy(asc(packs.name));

  return (
    <AdminUserClient
      session={{
        user: {
          id: session.user.id,
          email: session.user.email ?? "",
          role: session.user.role ?? "player",
        },
      }}
      user={user}
      memberships={memberships}
    />
  );
}
