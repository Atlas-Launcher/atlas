import { headers } from "next/headers";

import { auth } from "@/auth";
import InviteClient from "@/app/invite/invite-client";

interface InvitePageProps {
  searchParams: Promise<{
    code?: string;
  }>;
}

export default async function InvitePage({ searchParams }: InvitePageProps) {
  const { code } = await searchParams;
  const session = await auth.api.getSession({ headers: await headers() });
  const signedIn = Boolean(session?.user);

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto w-full max-w-5xl">
        <InviteClient code={code ?? null} signedIn={signedIn} />
      </div>
    </div>
  );
}
