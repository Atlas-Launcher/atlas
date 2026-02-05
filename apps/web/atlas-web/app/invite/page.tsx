import { headers } from "next/headers";

import { auth } from "@/auth";
import InviteClient from "@/app/invite/invite-client";

interface InvitePageProps {
  searchParams: Promise<{
    pack?: string;
  }>;
}

export default async function InvitePage({ searchParams }: InvitePageProps) {
  const { pack } = await searchParams;
  const session = await auth.api.getSession({ headers: await headers() });

  if (pack && session?.user) {
    return (
      <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
        <div className="mx-auto max-w-md">
          <InviteClient packId={pack} signedIn />
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <InviteClient packId={pack ?? null} signedIn={false} />
      </div>
    </div>
  );
}
