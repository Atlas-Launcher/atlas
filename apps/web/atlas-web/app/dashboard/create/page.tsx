import { headers } from "next/headers";
import { redirect } from "next/navigation";

import { auth } from "@/auth";
import CreatePackClient from "@/app/dashboard/create/create-pack-client";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import SignOutButton from "@/app/dashboard/sign-out-button";

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

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <CreatePackClient />
      </div>
    </div>
  );
}
