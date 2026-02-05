const roleCards = [
  {
    title: "System Admin",
    description:
      "Owns tenant health, issues creator invite codes, and enforces platform policy.",
    tags: ["Invite codes", "Policy controls", "Audit ledger"],
  },
  {
    title: "Pack Creator",
    description:
      "Connects GitHub repos, ships builds, and manages player access.",
    tags: ["Repo linking", "Deploy tokens", "Channel access"],
  },
  {
    title: "Player",
    description:
      "Joins packs through access links and syncs the right channel instantly.",
    tags: ["Access links", "Channel toggles", "Auto updates"],
  },
];

const workflowSteps = [
  {
    step: "01",
    title: "Import & Secure",
    description:
      "Creator links a GitHub repo, Hub injects CI workflow and deploy token secrets.",
  },
  {
    step: "02",
    title: "Compile & Upload",
    description:
      "GitHub Actions runs the builder CLI, producing a compressed binary blob.",
  },
  {
    step: "03",
    title: "Hydrate & Launch",
    description:
      "Launcher pulls the active channel, hydrates configs, verifies hashes, and starts the JVM.",
  },
];

const formatHighlights = [
  {
    title: "Single Binary Artifact",
    detail:
      "Zstd-compressed Bincode blob representing the exact build state for a commit.",
  },
  {
    title: "Virtual Filesystem Map",
    detail:
      "Embedded byte-map ships all configs, scripts, and small assets in one payload.",
  },
  {
    title: "Smart Dependency Manifest",
    detail:
      "External jars include URLs, hashes, and platform filters for lean hydration.",
  },
];

const channelCards = [
  {
    title: "Production",
    summary: "Stable releases with curated access.",
    status: "Pinned to QA-approved builds.",
  },
  {
    title: "Beta",
    summary: "Creator-reviewed upgrades for early adopters.",
    status: "Invite-key or explicit permission only.",
  },
  {
    title: "Dev",
    summary: "Auto-updates on every Git push.",
    status: "CI promotes latest build automatically.",
  },
];

const hubFeatures = [
  {
    title: "Release Manager",
    detail: "Move channel pointers across immutable builds instantly.",
  },
  {
    title: "Build Ledger",
    detail: "Every commit becomes a traceable artifact with timestamps.",
  },
  {
    title: "Invite Access",
    detail: "Tiered onboarding for creators and players with channel gating.",
  },
  {
    title: "Asset Storage",
    detail: "Cloudflare R2 hosts immutable blobs and custom mod jars.",
  },
];

const stackItems = [
  "Next.js (edge-ready)",
  "PostgreSQL metadata store",
  "Cloudflare R2 + CDN",
  "Rust CLI + Launcher",
  "Zstd + Bincode serialization",
];

export default function Home() {
  return (
    <div className="atlas-grid relative min-h-screen overflow-hidden bg-[var(--atlas-cream)] text-[var(--atlas-ink)]">
      <div
        className="pointer-events-none absolute -top-40 left-[-20%] h-[520px] w-[520px] rounded-full bg-[radial-gradient(circle,_rgba(60,132,109,0.35)_0%,_rgba(60,132,109,0)_70%)] blur-2xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute -top-32 right-[-10%] h-[420px] w-[420px] rounded-full bg-[radial-gradient(circle,_rgba(247,184,107,0.45)_0%,_rgba(247,184,107,0)_68%)] blur-3xl"
        aria-hidden="true"
      />
      <div
        className="pointer-events-none absolute bottom-0 left-1/2 h-[360px] w-[360px] -translate-x-1/2 rounded-full bg-[radial-gradient(circle,_rgba(122,173,201,0.35)_0%,_rgba(122,173,201,0)_70%)] blur-3xl"
        aria-hidden="true"
      />

      <header className="relative z-10 mx-auto flex w-full max-w-6xl items-center justify-between px-6 py-6">
        <div className="flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-[var(--atlas-ink)] text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)]">
            A
          </div>
          <div>
            <p className="text-sm font-semibold uppercase tracking-[0.35em] text-[var(--atlas-ink-muted)]">
              Atlas Hub
            </p>
            <p className="text-xs text-[var(--atlas-ink-muted)]">Source-in-Git, Distribution-in-Binary</p>
          </div>
        </div>
        <div className="hidden items-center gap-6 text-sm font-medium text-[var(--atlas-ink-muted)] md:flex">
          <span>Dashboard</span>
          <span>Releases</span>
          <span>Access</span>
          <span>Storage</span>
        </div>
        <div className="flex items-center gap-3">
          <button
            type="button"
            className="rounded-full border border-[var(--atlas-ink)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5 hover:bg-[var(--atlas-ink)] hover:text-[var(--atlas-cream)]"
          >
            Request Invite
          </button>
          <button
            type="button"
            className="rounded-full bg-[var(--atlas-accent)] px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] shadow-[0_10px_30px_rgba(60,132,109,0.25)] transition hover:-translate-y-0.5"
          >
            View Packs
          </button>
        </div>
      </header>

      <main className="relative z-10 mx-auto w-full max-w-6xl px-6 pb-24">
        <section className="grid gap-12 pb-16 pt-12 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="space-y-8">
            <div className="inline-flex items-center gap-2 rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
              High-performance modpack distribution
            </div>
            <h1 className="text-4xl font-semibold leading-tight text-[var(--atlas-ink)] md:text-6xl">
              Build once. Ship a single binary. Hydrate at game speed.
            </h1>
            <p className="max-w-2xl text-lg text-[var(--atlas-ink-muted)]">
              Atlas brings your modpacks together with a Git-native workflow, immutable build artifacts, and channel-based
              releases. The Hub manages access and promotion while the launcher handles zero-waste hydration.
            </p>
            <div className="flex flex-wrap gap-4">
              <button
                type="button"
                className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
              >
                Create a Pack
              </button>
              <button
                type="button"
                className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
              >
                Watch the Pipeline
              </button>
            </div>
            <div className="grid gap-4 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 backdrop-blur">
              <div className="flex items-center justify-between text-sm font-medium">
                <span className="text-[var(--atlas-ink-muted)]">Active Channel</span>
                <span className="rounded-full bg-[var(--atlas-accent)]/30 px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em]">
                  Dev
                </span>
              </div>
              <div className="grid gap-4 md:grid-cols-3">
                {[
                  { label: "Latest Build", value: "c9f1e7b" },
                  { label: "Hydration", value: "4.2s" },
                  { label: "Dependencies", value: "128 verified" },
                ].map((item) => (
                  <div key={item.label} className="space-y-1 rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                      {item.label}
                    </p>
                    <p className="text-lg font-semibold text-[var(--atlas-ink)]">{item.value}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div className="space-y-6">
            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-6 text-[var(--atlas-cream)] shadow-[0_30px_80px_rgba(15,23,42,0.25)]">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
                Release Manager
              </p>
              <h2 className="mt-4 text-2xl font-semibold">Channel Pointers</h2>
              <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
                Promote a tested build by moving pointers. Roll back instantly without rebuilding artifacts.
              </p>
              <div className="mt-6 space-y-3">
                {channelCards.map((channel) => (
                  <div
                    key={channel.title}
                    className="flex items-center justify-between rounded-2xl bg-white/10 px-4 py-3"
                  >
                    <div>
                      <p className="text-sm font-semibold text-[var(--atlas-cream)]">{channel.title}</p>
                      <p className="text-xs text-[var(--atlas-cream)]/60">{channel.summary}</p>
                    </div>
                    <span className="text-xs uppercase tracking-[0.2em] text-[var(--atlas-accent-light)]">
                      Live
                    </span>
                  </div>
                ))}
              </div>
            </div>

            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
                Distribution Format
              </p>
              <h3 className="mt-4 text-xl font-semibold text-[var(--atlas-ink)]">Librarian .bin</h3>
              <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
                A compressed, serialized byte-map with embedded configs and dependency manifests.
              </p>
              <div className="mt-5 space-y-3">
                {formatHighlights.map((highlight) => (
                  <div key={highlight.title} className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                    <p className="text-sm font-semibold text-[var(--atlas-ink)]">{highlight.title}</p>
                    <p className="text-xs text-[var(--atlas-ink-muted)]">{highlight.detail}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="grid gap-6 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
              Hub Capabilities
            </p>
            <h2 className="mt-3 text-3xl font-semibold">Control plane for creators and players</h2>
          </div>
          <div className="grid gap-4 md:grid-cols-2">
            {hubFeatures.map((feature) => (
              <div key={feature.title} className="rounded-2xl bg-[var(--atlas-cream)]/70 p-5">
                <p className="text-sm font-semibold text-[var(--atlas-ink)]">{feature.title}</p>
                <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{feature.detail}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="mt-16 grid gap-10 lg:grid-cols-[0.95fr_1.05fr]">
          <div className="space-y-6 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8">
            <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
              Core Workflow
            </p>
            <h2 className="text-3xl font-semibold">Source-in-Git, Distribution-in-Binary</h2>
            <p className="text-sm text-[var(--atlas-ink-muted)]">
              Every commit becomes a deterministic build. Channels stay mutable while artifacts remain immutable.
            </p>
            <div className="space-y-4">
              {workflowSteps.map((step) => (
                <div key={step.step} className="flex items-start gap-4 rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                  <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-[var(--atlas-ink)] text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)]">
                    {step.step}
                  </div>
                  <div>
                    <p className="text-sm font-semibold text-[var(--atlas-ink)]">{step.title}</p>
                    <p className="text-xs text-[var(--atlas-ink-muted)]">{step.description}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="space-y-6">
            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
                Identity & Access
              </p>
              <h2 className="mt-3 text-3xl font-semibold">Invite-only, channel-aware access</h2>
              <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
                Roles, encrypted tokens, and channel permission gates keep releases scoped to the right audiences.
              </p>
              <div className="mt-6 grid gap-4">
                {roleCards.map((role) => (
                  <div key={role.title} className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                    <p className="text-sm font-semibold text-[var(--atlas-ink)]">{role.title}</p>
                    <p className="mt-2 text-xs text-[var(--atlas-ink-muted)]">{role.description}</p>
                    <div className="mt-3 flex flex-wrap gap-2">
                      {role.tags.map((tag) => (
                        <span
                          key={tag}
                          className="rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-8 text-[var(--atlas-cream)]">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
                Infrastructure
              </p>
              <h3 className="mt-3 text-2xl font-semibold">Built for scale and speed</h3>
              <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
                Edge-ready APIs, immutable blobs, and fast Rust tooling keep hydration reliable under load.
              </p>
              <div className="mt-5 flex flex-wrap gap-2">
                {stackItems.map((item) => (
                  <span
                    key={item}
                    className="rounded-full border border-white/15 bg-white/10 px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
                  >
                    {item}
                  </span>
                ))}
              </div>
            </div>
          </div>
        </section>

        <section className="mt-16 grid gap-6 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 md:grid-cols-[0.9fr_1.1fr]">
          <div className="space-y-4">
            <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
              Channel Control
            </p>
            <h2 className="text-3xl font-semibold">Promote releases with confidence</h2>
            <p className="text-sm text-[var(--atlas-ink-muted)]">
              Build history stays immutable while channels update instantly. Rollbacks are just pointer flips.
            </p>
            <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4">
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Last Promotion
              </p>
              <p className="mt-2 text-sm text-[var(--atlas-ink)]">
                Beta {"->"} Production - Build 9b71d2f
              </p>
              <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">Triggered by Creator on 2026-02-04</p>
            </div>
          </div>
          <div className="grid gap-4 md:grid-cols-3">
            {channelCards.map((channel) => (
              <div key={channel.title} className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                <p className="text-sm font-semibold text-[var(--atlas-ink)]">{channel.title}</p>
                <p className="mt-2 text-xs text-[var(--atlas-ink-muted)]">{channel.summary}</p>
                <p className="mt-3 text-xs text-[var(--atlas-ink-muted)]">{channel.status}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="mt-16 grid gap-6 rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8">
          <div className="flex flex-wrap items-center justify-between gap-6">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
                Binary Hydration
              </p>
              <h2 className="mt-3 text-3xl font-semibold">Fast client sync with deterministic inputs</h2>
              <p className="mt-3 max-w-2xl text-sm text-[var(--atlas-ink-muted)]">
                Launcher hydration writes config files directly, filters platform-specific dependencies, and verifies every
                hash before launch. One blob equals one exact game state.
              </p>
            </div>
            <button
              type="button"
              className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)]"
            >
              View Launcher Logs
            </button>
          </div>
          <div className="grid gap-4 md:grid-cols-3">
            {[
              {
                label: "Zstd Level",
                value: "19",
                hint: "High compression with fast decode",
              },
              {
                label: "Average Payload",
                value: "1.2 GB",
                hint: "Compressed multi-mod pack",
              },
              {
                label: "Hash Verification",
                value: "SHA-256",
                hint: "Immutable dependency integrity",
              },
            ].map((item) => (
              <div key={item.label} className="rounded-2xl bg-[var(--atlas-cream)]/70 p-4">
                <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                  {item.label}
                </p>
                <p className="mt-2 text-lg font-semibold text-[var(--atlas-ink)]">{item.value}</p>
                <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">{item.hint}</p>
              </div>
            ))}
          </div>
        </section>

        <section className="mt-16 rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] px-8 py-10 text-[var(--atlas-cream)]">
          <div className="flex flex-wrap items-center justify-between gap-6">
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
                Ready to ship?
              </p>
              <h2 className="mt-3 text-3xl font-semibold">Stand up a new pack in minutes</h2>
              <p className="mt-2 text-sm text-[var(--atlas-cream)]/70">
                Connect a repo, push configs, and let the pipeline deliver a single binary build to every player.
              </p>
            </div>
            <button
              type="button"
              className="rounded-full bg-[var(--atlas-accent)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)]"
            >
              Launch Dashboard
            </button>
          </div>
        </section>
      </main>

      <footer className="relative z-10 border-t border-[var(--atlas-ink)]/10 bg-white/60">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-6 px-6 py-8 text-xs text-[var(--atlas-ink-muted)]">
          <span>Atlas Hub - Pack distribution control plane</span>
          <span>Immutable builds - Mutable channels - Deterministic hydration</span>
        </div>
      </footer>
    </div>
  );
}
