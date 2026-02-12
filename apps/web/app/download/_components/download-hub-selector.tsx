"use client";

import Link from "next/link";
import { useState } from "react";

export type DownloadHubProduct = {
  id: string;
  label: string;
  detail: string;
  latest: string;
  primaryHref: string;
  primaryLabel: string;
  pageHref: string;
  pageLabel: string;
  installSteps: string[];
};

export function DownloadHubSelector({
  products,
  defaultProductId,
}: {
  products: DownloadHubProduct[];
  defaultProductId: string;
}) {
  const [selectedId, setSelectedId] = useState(defaultProductId);
  const selected = products.find((product) => product.id === selectedId) ?? products[0];

  return (
    <div className="space-y-12 pt-10">
      <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
        <div className="mx-auto max-w-4xl text-center">
          <p className="inline-flex items-center rounded-full border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            Atlas downloads
          </p>
          <h1 className="mt-4 text-4xl font-semibold leading-tight md:text-6xl">Choose your Atlas install path</h1>
          <p className="mx-auto mt-4 max-w-3xl text-lg text-[var(--atlas-ink-muted)]">
            Pick a product first, then follow the guided install flow for your platform.
          </p>
        </div>

        <div className="mx-auto mt-8 grid max-w-5xl gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {products.map((product) => {
            const isActive = product.id === selected.id;
            return (
              <button
                key={product.id}
                type="button"
                onClick={() => setSelectedId(product.id)}
                className={`rounded-2xl border px-4 py-4 text-left transition ${
                  isActive
                    ? "border-[var(--atlas-ink)] bg-[var(--atlas-ink)] text-[var(--atlas-cream)]"
                    : "border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 text-[var(--atlas-ink)] hover:border-[var(--atlas-ink)]/30"
                }`}
              >
                <p className="text-sm font-semibold">{product.label}</p>
                <p className={`mt-2 text-xs ${isActive ? "text-[var(--atlas-cream)]/80" : "text-[var(--atlas-ink-muted)]"}`}>
                  {product.detail}
                </p>
                <p className={`mt-2 text-xs ${isActive ? "text-[var(--atlas-cream)]/70" : "text-[var(--atlas-ink-muted)]"}`}>
                  {product.latest}
                </p>
              </button>
            );
          })}
        </div>

        <div className="mx-auto mt-8 max-w-2xl text-center">
          <a
            href={selected.primaryHref}
            className="inline-flex rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
          >
            {selected.primaryLabel}
          </a>
          <div className="mt-4">
            <Link
              href={selected.pageHref}
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
            >
              {selected.pageLabel}
            </Link>
          </div>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <h2 className="text-2xl font-semibold text-[var(--atlas-ink)]">{`Install ${selected.label}`}</h2>
          <ol className="mt-4 space-y-3 text-sm text-[var(--atlas-ink-muted)]">
            {selected.installSteps.map((step, index) => (
              <li key={`${selected.id}-step-${index + 1}`}>{`${index + 1}. ${step}`}</li>
            ))}
          </ol>
        </div>

        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-6 text-[var(--atlas-cream)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
            Quick tips
          </p>
          <ul className="mt-4 space-y-2 text-sm text-[var(--atlas-cream)]/80">
            <li>Use stable releases for production installs.</li>
            <li>Open product pages for manual files and platform specifics.</li>
            <li>Runner on Windows should run through WSL.</li>
          </ul>
        </div>
      </section>
    </div>
  );
}
