import type { ReactNode } from "react";

import PublicNavbar from "@/components/public-navbar";

export default function DownloadLayout({ children }: { children: ReactNode }) {
  return (
    <div className="atlas-grid relative min-h-screen overflow-hidden bg-[var(--atlas-cream)] text-[var(--atlas-ink)]">
      <div
        className="pointer-events-none absolute -top-40 left-[-25%] h-[520px] w-[520px] rounded-full bg-[radial-gradient(circle,_rgba(120,198,163,0.25)_0%,_rgba(120,198,163,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute -top-24 right-[-12%] h-[420px] w-[420px] rounded-full bg-[radial-gradient(circle,_rgba(169,201,231,0.32)_0%,_rgba(169,201,231,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute bottom-[-120px] left-1/2 h-[360px] w-[360px] -translate-x-1/2 rounded-full bg-[radial-gradient(circle,_rgba(244,214,160,0.35)_0%,_rgba(244,214,160,0)_70%)] blur-3xl"
        aria-hidden="true"
      />

      <PublicNavbar />

      <main className="relative z-10 mx-auto w-full max-w-6xl px-6 pb-24">{children}</main>

      <footer className="relative z-10 border-t border-[var(--atlas-ink)]/10 bg-white/60">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-6 px-6 py-8 text-xs text-[var(--atlas-ink-muted)]">
          <span>Atlas Hub downloads</span>
          <span>Fast installs. Reliable updates.</span>
        </div>
      </footer>
    </div>
  );
}
