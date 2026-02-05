import PublicNavbar from "@/components/public-navbar";

const workflowSteps = [
  {
    step: "01",
    title: "Create & Package",
    description:
      "Bring your modpack together with the files, settings, and assets you want players to get.",
  },
  {
    step: "02",
    title: "Review & Release",
    description:
      "Preview changes, invite testers, and choose when a build goes live.",
  },
  {
    step: "03",
    title: "Players Launch",
    description:
      "Players install quickly and always land on the right version.",
  },
];

const formatHighlights = [
  {
    title: "All-in-One Snapshot",
    detail:
      "Configs, scripts, and assets stay together so installs are consistent.",
  },
  {
    title: "Version Confidence",
    detail: "Every update is tracked so players get the exact build you approved.",
  },
  {
    title: "Smarter Downloads",
    detail: "Players only grab what they need for their device and platform.",
  },
];

const hubFeatures = [
  {
    title: "Release Staging",
    detail: "Move from testing to live without disrupting current players.",
  },
  {
    title: "Version History",
    detail: "See what changed, when it shipped, and who has access.",
  },
  {
    title: "Invite Access",
    detail: "Bring creators, admins, and players in with simple invites.",
  },
  {
    title: "Assets & Add-ons",
    detail: "Keep custom mods and extras organized alongside your pack.",
  },
];

const reliabilityPills = [
  "Fast updates",
  "Consistent installs",
  "Secure access",
  "Global delivery",
  "Built to scale",
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

      <PublicNavbar />

      <main className="relative z-10 mx-auto w-full max-w-6xl px-6 pb-24">
        <section className="grid gap-12 pb-16 pt-12 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="space-y-8">
            <div className="inline-flex items-center gap-2 rounded-full border border-[var(--atlas-ink)]/10 bg-white/70 px-4 py-2 text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
              Creator-friendly modpack delivery
            </div>
            <h1 className="text-4xl font-semibold leading-tight text-[var(--atlas-ink)] md:text-6xl">
              Build once. Share everywhere. Play fast.
            </h1>
            <p className="max-w-2xl text-lg text-[var(--atlas-ink-muted)]">
              Atlas brings your modpacks together with simple releases, reliable installs, and a hub that keeps creators
              and players in sync.
            </p>
            <div className="flex flex-wrap gap-4">
              <a
                href="/dashboard"
                className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
              >
                Create a Pack
              </a>
              <a
                href="/download"
                className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
              >
                View Downloads
              </a>
            </div>
          </div>

          <div className="space-y-6">
            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
                Pack Snapshot
              </p>
              <h3 className="mt-4 text-xl font-semibold text-[var(--atlas-ink)]">Everything in one place</h3>
              <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
                Keep your pack tidy and consistent, from configs to custom assets.
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
            <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-6 text-[var(--atlas-cream)] shadow-[0_30px_80px_rgba(15,23,42,0.25)]">
              <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
                Release Manager
              </p>
              <h2 className="mt-4 text-2xl font-semibold">Release Channels</h2>
              <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
                Move a tested build live in seconds, or roll back just as fast.
              </p>
              <div className="mt-5 flex flex-wrap gap-2">
                {["Dev", "Beta", "Production"].map((label) => (
                  <span
                    key={label}
                    className="rounded-full border border-white/15 bg-white/10 px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
                  >
                    {label}
                  </span>
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
            <h2 className="mt-3 text-3xl font-semibold">A shared space for creators and players</h2>
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
            <h2 className="text-3xl font-semibold">From build to play in three steps</h2>
            <p className="text-sm text-[var(--atlas-ink-muted)]">
              Keep your team aligned while players always launch the right version.
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

          <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-ink)] p-8 text-[var(--atlas-cream)]">
            <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-accent-light)]">
              Reliability
            </p>
            <h3 className="mt-3 text-2xl font-semibold">Built for stable launches</h3>
            <p className="mt-3 text-sm text-[var(--atlas-cream)]/70">
              Keep releases smooth, updates quick, and players confident in every install.
            </p>
            <div className="mt-5 flex flex-wrap gap-2">
              {reliabilityPills.map((item) => (
                <span
                  key={item}
                  className="rounded-full border border-white/15 bg-white/10 px-3 py-1 text-[10px] font-semibold uppercase tracking-[0.2em]"
                >
                  {item}
                </span>
              ))}
            </div>
          </div>
        </section>
      </main>

      <footer className="relative z-10 border-t border-[var(--atlas-ink)]/10 bg-white/60">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-6 px-6 py-8 text-xs text-[var(--atlas-ink-muted)]">
          <span>Atlas Hub - Modpacks made simple</span>
          <span>Build with confidence. Share with ease.</span>
        </div>
      </footer>
    </div>
  );
}
