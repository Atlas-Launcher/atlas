import { headers } from "next/headers";
import { redirect } from "next/navigation";

import { auth } from "@/auth";
import DashboardClient from "@/app/dashboard/dashboard-client";

export default async function DashboardPage() {
  const session = await auth.api.getSession({ headers: await headers() });

  if (!session?.user) {
    redirect("/sign-in");
  }

  return (
    <DashboardClient
      session={{
        user: {
          id: session.user.id,
          email: session.user.email ?? "",
          role: session.user.role ?? "player",
        },
      }}
    />
  );
}
