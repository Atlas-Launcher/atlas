import { Suspense } from "react";

import ConsentClient from "./consent-client";

export default function ConsentPage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen bg-transparent px-6 py-16 text-[var(--atlas-ink)]">
          <div className="mx-auto max-w-md">
            <div className="rounded-3xl border border-[var(--atlas-border)] bg-white p-8 shadow-sm">
              <p className="text-sm text-[var(--atlas-ink-muted)]">Loading consent…</p>
            </div>
          </div>
        </div>
      }
    >
      <ConsentClient />
    </Suspense>
  );
}
