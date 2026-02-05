import { headers } from "next/headers";
import { redirect } from "next/navigation";

import { auth } from "@/auth";
import PackDashboardClient from "@/app/dashboard/pack-dashboard-client";

interface PageProps {
  params: Promise<{
    packId: string;
  }>;
}

export default async function PackDashboardPage({ params }: PageProps) {
  const session = await auth.api.getSession({ headers: await headers() });

  if (!session?.user) {
    redirect("/sign-in");
  }

  const { packId } = await params;

  return (
    <PackDashboardClient
      packId={packId}
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
