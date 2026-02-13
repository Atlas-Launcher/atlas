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
      <section className="atlas-panel rounded-3xl p-8 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
        <div className="mx-auto max-w-4xl text-center">
          <p className="inline-flex items-center rounded-full border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            Atlas downloads
          </p>
          <h1 className="mt-4 text-4xl font-semibold leading-tight md:text-6xl">Choose what you want to install</h1>
          <p className="mx-auto mt-4 max-w-3xl text-lg text-[var(--atlas-ink-muted)]">
            Pick a product, then follow the guided steps for your platform.
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
                    ? "border-[hsl(var(--border)/0.95)] bg-[var(--atlas-inverse-bg)] text-[var(--atlas-inverse-fg)]"
                    : "border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] text-[var(--atlas-ink)] hover:border-[hsl(var(--border))]"
                }`}
              >
                <p className="text-sm font-semibold">{product.label}</p>
                <p className={`mt-2 text-xs ${isActive ? "text-[var(--atlas-inverse-muted)]" : "text-[var(--atlas-ink-muted)]"}`}>
                  {product.detail}
                </p>
                <p className={`mt-2 text-xs ${isActive ? "text-[var(--atlas-inverse-muted)]" : "text-[var(--atlas-ink-muted)]"}`}>
                  {product.latest}
                </p>
              </button>
            );
          })}
        </div>

        <div className="mx-auto mt-8 max-w-2xl text-center">
          <a
            href={selected.primaryHref}
            className="inline-flex rounded-full bg-[var(--atlas-inverse-bg)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-inverse-fg)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
          >
            {selected.primaryLabel}
          </a>
          <div className="mt-4 text-xs text-[var(--atlas-ink-muted)]">
            Need more options?
            <Link
              href={selected.pageHref}
              className="ml-1 underline underline-offset-4"
            >
              {selected.pageLabel}
            </Link>
          </div>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="atlas-panel rounded-3xl p-6">
          <h2 className="text-2xl font-semibold text-[var(--atlas-ink)]">{`Install ${selected.label}`}</h2>
          <ol className="mt-4 space-y-3 text-sm text-[var(--atlas-ink-muted)]">
            {selected.installSteps.map((step, index) => (
              <li key={`${selected.id}-step-${index + 1}`}>{`${index + 1}. ${step}`}</li>
            ))}
          </ol>
        </div>

        <div className="rounded-3xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-inverse-bg)] p-6 text-[var(--atlas-inverse-fg)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
            Quick tips
          </p>
          <ul className="mt-4 space-y-2 text-sm text-[var(--atlas-inverse-muted)]">
            <li>Use stable releases for production installs.</li>
            <li>Open product pages for manual files and platform specifics.</li>
            <li>Runner on Windows should run through WSL.</li>
          </ul>
        </div>
      </section>
    </div>
  );
}
