import { headers } from "next/headers";
import { redirect } from "next/navigation";
import { and, eq } from "drizzle-orm";

import { auth } from "@/auth";
import CreatePackClient from "@/app/dashboard/create/create-pack-client";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import SignOutButton from "@/app/dashboard/sign-out-button";
import { db } from "@/lib/db";
import { accounts } from "@/lib/db/schema";

export default async function CreatePackPage() {
  const session = await auth.api.getSession({ headers: await headers() });

  if (!session?.user) {
    redirect("/sign-in");
  }

  const role = session.user.role ?? "player";
  const canCreate = role === "admin" || role === "creator";

  if (!canCreate) {
    redirect("/dashboard");
  }

  const [githubAccount] = await db
    .select({ id: accounts.id })
    .from(accounts)
    .where(and(eq(accounts.userId, session.user.id), eq(accounts.providerId, "github")))
    .limit(1);

  if (!githubAccount) {
    redirect("/dashboard?tab=account&focus=github&next=/dashboard/create");
  }

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <CreatePackClient />
      </div>
    </div>
  );
}
