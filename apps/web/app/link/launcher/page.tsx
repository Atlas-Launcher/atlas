import { headers } from "next/headers";

import { auth } from "@/auth";
import LauncherLinkClient from "@/app/link/launcher/link-client";

interface LauncherLinkPageProps {
  searchParams: Promise<{
    code?: string;
    status?: string;
    message?: string;
  }>;
}

export default async function LauncherLinkPage({ searchParams }: LauncherLinkPageProps) {
  const { code } = await searchParams;
  const session = await auth.api.getSession({ headers: await headers() });
  const signedIn = Boolean(session?.user);

  return (
    <div className="min-h-screen bg-transparent px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto w-full max-w-3xl">
        <LauncherLinkClient code={code ?? null} signedIn={signedIn} />
      </div>
    </div>
  );
}
