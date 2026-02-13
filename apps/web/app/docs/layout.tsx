import type { ReactNode } from "react";

import PublicNavbar from "@/components/public-navbar";

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="atlas-app-shell atlas-grid relative min-h-screen overflow-hidden">
      <div
        className="pointer-events-none absolute -top-40 left-[-25%] h-[520px] w-[520px] rounded-full bg-[radial-gradient(circle,_rgba(120,198,163,0.2)_0%,_rgba(120,198,163,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute -top-24 right-[-12%] h-[420px] w-[420px] rounded-full bg-[radial-gradient(circle,_rgba(169,201,231,0.24)_0%,_rgba(169,201,231,0)_70%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute bottom-[-120px] left-1/2 h-[360px] w-[360px] -translate-x-1/2 rounded-full bg-[radial-gradient(circle,_rgba(244,214,160,0.22)_0%,_rgba(244,214,160,0)_70%)] blur-3xl"
        aria-hidden="true"
      />

      <PublicNavbar />

      <main className="relative z-10 mx-auto w-full max-w-[1440px] px-4 pb-16 sm:px-6 lg:px-8">{children}</main>

      <footer className="atlas-footer relative z-10">
        <div className="mx-auto flex w-full max-w-[1440px] flex-wrap items-center justify-between gap-6 px-4 py-6 text-xs text-[var(--atlas-ink-muted)] sm:px-6 lg:px-8">
          <span>Atlas Hub docs</span>
          <span>Clear guides for players, creators, and server hosts.</span>
        </div>
      </footer>
    </div>
  );
}
