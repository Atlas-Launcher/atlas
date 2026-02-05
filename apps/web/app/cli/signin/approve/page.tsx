import { Suspense } from "react";

import CliApproveClient from "./cli-approve-client";

export default function CliApprovePage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
          <div className="mx-auto max-w-md">
            <div className="rounded-3xl border border-[var(--atlas-border)] bg-white p-8 shadow-sm">
              <p className="text-sm text-[var(--atlas-ink-muted)]">Loading approval…</p>
            </div>
          </div>
        </div>
      }
    >
      <CliApproveClient />
    </Suspense>
  );
}
