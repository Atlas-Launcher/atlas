import { Suspense } from "react";

import CliSigninClient from "./cli-signin-client";

export default function CliSigninPage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
          <div className="mx-auto max-w-md">
            <div className="rounded-3xl border border-[var(--atlas-border)] bg-white p-8 shadow-sm">
              <p className="text-sm text-[var(--atlas-ink-muted)]">Loading CLI sign-in…</p>
            </div>
          </div>
        </div>
      }
    >
      <CliSigninClient />
    </Suspense>
  );
}
