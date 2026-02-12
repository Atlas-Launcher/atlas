"use client";

import { useState } from "react";

import { formatBytes } from "@/app/download/_components/shared";

export type GuidedManualDownload = {
  id: string;
  label: string;
  detail: string;
  href: string;
  size: number;
};

export type GuidedAction =
  | {
      type: "download";
      label: string;
      href: string;
      note?: string;
    }
  | {
      type: "command";
      label: string;
      command: string;
      note?: string;
    }
  | {
      type: "info";
      label: string;
      note: string;
    };

export type GuidedPlatform = {
  id: string;
  label: string;
  detail: string;
  action: GuidedAction;
  installTitle: string;
  installSteps: string[];
  manualTitle: string;
  manualDownloads: GuidedManualDownload[];
  manualEmptyLabel: string;
  nextSteps: string[];
};

export function PlatformGuidedDownload({
  badge,
  title,
  subtitle,
  latestLabel,
  platforms,
  defaultPlatformId,
}: {
  badge: string;
  title: string;
  subtitle: string;
  latestLabel: string;
  platforms: GuidedPlatform[];
  defaultPlatformId: string;
}) {
  const [selectedId, setSelectedId] = useState(defaultPlatformId);
  const [copyState, setCopyState] = useState<"idle" | "ok" | "error">("idle");
  const selected =
    platforms.find((platform) => platform.id === selectedId) ?? platforms[0];

  const copyWithFallback = (value: string) => {
    const textarea = document.createElement("textarea");
    textarea.value = value;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.top = "-9999px";
    textarea.style.left = "-9999px";
    document.body.appendChild(textarea);
    textarea.select();
    textarea.setSelectionRange(0, textarea.value.length);
    let ok = false;
    try {
      ok = document.execCommand("copy");
    } catch {
      ok = false;
    }
    document.body.removeChild(textarea);
    return ok;
  };

  const copyCommand = async () => {
    if (selected?.action.type !== "command") {
      return;
    }

    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(selected.action.command);
      } else if (!copyWithFallback(selected.action.command)) {
        throw new Error("Clipboard API unavailable");
      }
      setCopyState("ok");
    } catch {
      setCopyState(copyWithFallback(selected.action.command) ? "ok" : "error");
    }

    window.setTimeout(() => setCopyState("idle"), 1500);
  };

  return (
    <div className="space-y-12 pt-10">
      <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 shadow-[0_24px_60px_rgba(16,20,24,0.1)]">
        <div className="mx-auto max-w-4xl text-center">
          <p className="inline-flex items-center rounded-full border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
            {badge}
          </p>
          <h1 className="mt-4 text-4xl font-semibold leading-tight md:text-6xl">{title}</h1>
          <p className="mx-auto mt-4 max-w-3xl text-lg text-[var(--atlas-ink-muted)]">{subtitle}</p>
          <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">{latestLabel}</p>
        </div>

        <div className="mx-auto mt-8 grid max-w-5xl gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {platforms.map((platform) => {
            const isActive = platform.id === selected.id;
            return (
              <button
                key={platform.id}
                type="button"
                onClick={() => {
                  setSelectedId(platform.id);
                  setCopyState("idle");
                }}
                className={`rounded-2xl border px-4 py-4 text-left transition ${
                  isActive
                    ? "border-[var(--atlas-ink)] bg-[var(--atlas-ink)] text-[var(--atlas-cream)]"
                    : "border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 text-[var(--atlas-ink)] hover:border-[var(--atlas-ink)]/30"
                }`}
              >
                <p className="text-sm font-semibold">{platform.label}</p>
                <p className={`mt-1 text-xs ${isActive ? "text-[var(--atlas-cream)]/80" : "text-[var(--atlas-ink-muted)]"}`}>
                  {platform.detail}
                </p>
              </button>
            );
          })}
        </div>

        <div className="mx-auto mt-8 max-w-2xl text-center">
          {selected.action.type === "download" ? (
            <>
              <a
                href={selected.action.href}
                className="inline-flex rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
              >
                {selected.action.label}
              </a>
              {selected.action.note ? (
                <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">{selected.action.note}</p>
              ) : null}
            </>
          ) : null}

          {selected.action.type === "command" ? (
            <>
              <p className="text-lg font-semibold text-[var(--atlas-ink)]">{selected.action.label}</p>
              <div className="mt-3 flex items-center justify-center gap-2 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-3 text-left">
                <code className="block flex-1 overflow-x-auto text-xs text-[var(--atlas-ink)]">
                  {selected.action.command}
                </code>
                <button
                  type="button"
                  onClick={copyCommand}
                  className={`rounded-lg border px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.15em] ${
                    copyState === "ok"
                      ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-700"
                      : copyState === "error"
                        ? "border-red-500/40 bg-red-500/10 text-red-700"
                        : "border-[var(--atlas-ink)]/20 text-[var(--atlas-ink)]"
                  }`}
                >
                  {copyState === "ok" ? "Copied" : copyState === "error" ? "Copy failed" : "Copy"}
                </button>
              </div>
              {copyState !== "idle" ? (
                <p
                  aria-live="polite"
                  className={`mt-2 text-xs ${
                    copyState === "ok" ? "text-emerald-700" : "text-red-700"
                  }`}
                >
                  {copyState === "ok"
                    ? "Command copied to clipboard."
                    : "Could not copy automatically. Please copy the command manually."}
                </p>
              ) : null}
              {selected.action.note ? (
                <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">{selected.action.note}</p>
              ) : null}
            </>
          ) : null}

          {selected.action.type === "info" ? (
            <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-sm text-[var(--atlas-ink-muted)]">
              <p className="font-semibold text-[var(--atlas-ink)]">{selected.action.label}</p>
              <p className="mt-2">{selected.action.note}</p>
            </div>
          ) : null}
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <h2 className="text-2xl font-semibold text-[var(--atlas-ink)]">{selected.installTitle}</h2>
          <ol className="mt-4 space-y-3 text-sm text-[var(--atlas-ink-muted)]">
            {selected.installSteps.map((step, index) => (
              <li key={`${selected.id}-step-${index + 1}`}>{`${index + 1}. ${step}`}</li>
            ))}
          </ol>
        </div>

        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-6 text-[var(--atlas-cream)]">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
            Next steps
          </p>
          <ul className="mt-4 space-y-2 text-sm text-[var(--atlas-cream)]/80">
            {selected.nextSteps.map((step, index) => (
              <li key={`${selected.id}-next-${index + 1}`}>{step}</li>
            ))}
          </ul>
        </div>
      </section>

      <section className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
        <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
          {selected.manualTitle}
        </p>
        <div className="mt-4 grid gap-3">
          {selected.manualDownloads.length ? (
            selected.manualDownloads.map((asset) => (
              <a
                key={asset.id}
                href={asset.href}
                className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-[var(--atlas-ink)] transition hover:border-[var(--atlas-ink)]"
                rel="noreferrer"
                target="_blank"
              >
                <span>
                  <span className="block text-sm font-medium">{asset.label}</span>
                  <span className="block text-xs text-[var(--atlas-ink-muted)]">{asset.detail}</span>
                </span>
                <span className="text-xs text-[var(--atlas-ink-muted)]">{formatBytes(asset.size)}</span>
              </a>
            ))
          ) : (
            <span className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
              {selected.manualEmptyLabel}
            </span>
          )}
        </div>
      </section>
    </div>
  );
}
