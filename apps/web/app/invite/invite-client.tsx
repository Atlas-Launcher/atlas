"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";

import { authClient } from "@/lib/auth-client";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { playerWebCopy } from "@/app/_copy/player";

type InviteStep = "account" | "launcher" | "done";

interface InviteClientProps {
  code: string | null;
  signedIn: boolean;
}

type InvitePreview = {
  pack: {
    id: string;
    name: string;
  };
  creator: {
    name: string | null;
    email: string | null;
  };
};

type InviteStatus = "idle" | "loading" | "accepted" | "warning";
type RecommendedChannel = "dev" | "beta" | "production";

type InviteAcceptResponse = {
  success: boolean;
  packId: string;
  pack?: {
    id: string;
    name: string;
    slug: string;
  };
  onboarding?: {
    deepLink: string;
    recommendedChannel: RecommendedChannel;
  };
};

const steps: { id: InviteStep; label: string; detail: string }[] = [
  {
    id: "account",
    label: "Create account",
    detail: "Set up your Atlas login.",
  },
  {
    id: "launcher",
    label: "Open launcher",
    detail: "Open Atlas Launcher and land on your invited pack.",
  },
  { id: "done", label: "Continue", detail: "Finish setup in Atlas Launcher." },
];

function resolveInviteStep(value: string | null): InviteStep | null {
  if (value === "account" || value === "launcher" || value === "done") {
    return value;
  }
  if (value === "setup") {
    return "launcher";
  }
  return null;
}

export default function InviteClient({ code, signedIn }: InviteClientProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const launcherFallbackTimer = useRef<number | null>(null);
  const launcherDownloadRef = useRef<HTMLAnchorElement | null>(null);

  const [preview, setPreview] = useState<InvitePreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [signedInState, setSignedInState] = useState(signedIn);
  const [inviteStatus, setInviteStatus] = useState<InviteStatus>("idle");
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [inviteAcceptResponse, setInviteAcceptResponse] = useState<InviteAcceptResponse | null>(null);
  const [launcherFallbackVisible, setLauncherFallbackVisible] = useState(false);
  const [launcherAttempted, setLauncherAttempted] = useState(false);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [accountLoading, setAccountLoading] = useState(false);
  const [accountError, setAccountError] = useState<string | null>(null);

  const stepParam = searchParams.get("step");
  const initialStep = useMemo<InviteStep>(() => {
    const resolved = resolveInviteStep(stepParam);
    if (resolved) return resolved;
    if (!signedIn) return "account";
    return "launcher";
  }, [signedIn, stepParam]);

  const [manualStep, setManualStep] = useState<InviteStep>(initialStep);

  const resolvedStep = useMemo<InviteStep>(() => {
    return resolveInviteStep(stepParam) ?? manualStep;
  }, [manualStep, stepParam]);

  const deepLink = useMemo(() => {
    const fromAccept = inviteAcceptResponse?.onboarding?.deepLink;
    if (fromAccept) {
      return fromAccept;
    }

    const packId = inviteAcceptResponse?.packId ?? preview?.pack.id;
    if (!packId) {
      return null;
    }

    return `atlas://onboarding?source=invite&packId=${encodeURIComponent(packId)}&channel=production`;
  }, [inviteAcceptResponse, preview?.pack.id]);

  useEffect(() => {
    return () => {
      if (launcherFallbackTimer.current) {
        window.clearTimeout(launcherFallbackTimer.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!launcherFallbackVisible) {
      return;
    }
    launcherDownloadRef.current?.focus();
  }, [launcherFallbackVisible]);

  useEffect(() => {
    if (!code) {
      return;
    }
    const loadPreview = async () => {
      setPreviewError(null);
      const response = await fetch(`/api/v1/invites/preview?code=${encodeURIComponent(code)}`);
      const data = await response.json();
      if (!response.ok) {
        setPreviewError(data?.error ?? "Unable to load invite details.");
        return;
      }
      setPreview(data);
    };
    loadPreview().catch(() => setPreviewError("Unable to load invite details."));
  }, [code]);

  useEffect(() => {
    if (!signedInState || !code || inviteStatus !== "idle") {
      return;
    }

    const acceptInvite = async () => {
      setInviteStatus("loading");
      setInviteError(null);
      const response = await fetch("/api/v1/invites/accept", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });
      const data = (await response.json()) as InviteAcceptResponse & { error?: string };

      if (!response.ok) {
        setInviteStatus("warning");
        setInviteError(data?.error ?? "Unable to accept invite.");
        return;
      }

      if (!data?.packId) {
        setInviteStatus("warning");
        setInviteError("Invite accepted, but destination pack was missing.");
        return;
      }

      setInviteAcceptResponse(data);
      setInviteStatus("accepted");
    };

    acceptInvite().catch(() => {
      setInviteStatus("warning");
      setInviteError("Unable to accept invite.");
    });
  }, [code, inviteStatus, signedInState]);

  const setStepAndUrl = useCallback(
    (nextStep: InviteStep) => {
      if (!code) {
        setManualStep(nextStep);
        return;
      }
      const params = new URLSearchParams(searchParams.toString());
      params.set("code", code);
      params.set("step", nextStep);
      setManualStep(nextStep);
      router.replace(`/invite?${params.toString()}`);
    },
    [code, router, searchParams]
  );

  useEffect(() => {
    if (inviteStatus !== "accepted" || resolvedStep !== "account" || !code) {
      return;
    }
    const params = new URLSearchParams(searchParams.toString());
    params.set("code", code);
    params.set("step", "launcher");
    router.replace(`/invite?${params.toString()}`);
  }, [code, inviteStatus, resolvedStep, router, searchParams]);

  const accountComplete = signedInState;

  const handleSignUp = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (accountLoading) {
      return;
    }
    setAccountError(null);
    setAccountLoading(true);
    const resolvedName = name.trim() || email.split("@")[0] || "Player";
    const result = await authClient.signUp.email({
      email,
      password,
      name: resolvedName,
      role: "player",
    });
    setAccountLoading(false);

    if (result?.error) {
      setAccountError(result.error.message ?? "Unable to create account.");
      return;
    }

    setSignedInState(true);
    setInviteStatus("idle");
  };

  const handleOpenLauncher = () => {
    if (!deepLink) {
      setLauncherFallbackVisible(true);
      return;
    }

    setLauncherAttempted(true);
    setLauncherFallbackVisible(false);
    if (launcherFallbackTimer.current) {
      window.clearTimeout(launcherFallbackTimer.current);
    }

    window.location.href = deepLink;

    launcherFallbackTimer.current = window.setTimeout(() => {
      setLauncherFallbackVisible(true);
    }, 1500);
  };

  if (!code) {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Invite</Badge>
          <CardTitle>Missing invite code</CardTitle>
          <CardDescription>Invite links require a code.</CardDescription>
        </CardHeader>
        <CardContent>
          <Link href="/sign-in">
            <Button>Back to sign in</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  if (previewError) {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Invite</Badge>
          <CardTitle>Invite not available</CardTitle>
          <CardDescription>{previewError}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-3">
          <Link href="/sign-in">
            <Button>Sign in</Button>
          </Link>
          <Link href="/sign-in">
            <Button variant="outline">Request a new invite</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="grid gap-10 lg:grid-cols-[0.9fr_1.1fr]">
      <aside className="space-y-6">
        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6 shadow-[0_20px_50px_rgba(16,20,24,0.1)]">
          <Badge variant="secondary">Invite</Badge>
          <h2 className="mt-4 text-3xl font-semibold">{playerWebCopy.invite.title}</h2>
          <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
            {preview?.pack?.name
              ? `You're joining ${preview.pack.name}. We'll connect your account and send you to Atlas Launcher.`
              : "We'll connect your account and send you to Atlas Launcher."}
          </p>
          {preview?.pack?.name ? (
            <div className="mt-5 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
              <p className="text-sm font-semibold text-[var(--atlas-ink)]">{preview.pack.name}</p>
              <p className="mt-1">
                {preview.creator?.name ? `Created by ${preview.creator.name}` : "Pack creator"}
              </p>
            </div>
          ) : null}
        </div>

        <div className="rounded-3xl border border-[var(--atlas-ink)]/10 bg-white/70 p-6">
          <p className="text-xs font-semibold uppercase tracking-[0.3em] text-[var(--atlas-ink-muted)]">
            Onboarding steps
          </p>
          <div className="mt-4 space-y-3">
            {steps.map((item, index) => {
              const isActive = item.id === resolvedStep;
              const isComplete =
                steps.findIndex((stepItem) => stepItem.id === resolvedStep) > index;
              return (
                <div
                  key={item.id}
                  className={`rounded-2xl border px-4 py-3 ${
                    isActive
                      ? "border-[var(--atlas-ink)] bg-[var(--atlas-cream)]"
                      : isComplete
                        ? "border-[var(--atlas-ink)]/20 bg-white/60"
                        : "border-[var(--atlas-ink)]/10 bg-white/40"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className={`flex h-8 w-8 items-center justify-center rounded-full text-xs font-semibold ${
                        isComplete
                          ? "bg-[var(--atlas-ink)] text-[var(--atlas-cream)]"
                          : isActive
                            ? "border border-[var(--atlas-ink)] text-[var(--atlas-ink)]"
                            : "border border-[var(--atlas-ink)]/20 text-[var(--atlas-ink-muted)]"
                      }`}
                    >
                      {index + 1}
                    </div>
                    <div>
                      <p className="text-sm font-semibold text-[var(--atlas-ink)]">{item.label}</p>
                      <p className="text-xs text-[var(--atlas-ink-muted)]">{item.detail}</p>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </aside>

      <section className="space-y-6">
        {resolvedStep === "account" ? (
          <Card>
            <CardHeader>
              <CardTitle>{playerWebCopy.invite.accountStepTitle}</CardTitle>
              <CardDescription>We will accept your invite as soon as you are signed in.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {accountComplete ? (
                <div className="space-y-4">
                  <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                    Account ready. Connecting your invite now.
                  </div>
                  {inviteStatus === "loading" ? (
                    <p className="text-xs text-[var(--atlas-ink-muted)]">Accepting invite...</p>
                  ) : null}
                  {inviteStatus === "accepted" ? (
                    <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                      Invite accepted. You are in the pack.
                    </div>
                  ) : null}
                  {inviteStatus === "warning" ? (
                    <div
                      className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700"
                      role="alert"
                    >
                      {inviteError ?? "Invite acceptance needs attention."}
                    </div>
                  ) : null}
                  <div className="flex flex-wrap gap-3">
                    <Button
                      onClick={() => setStepAndUrl("launcher")}
                      disabled={inviteStatus === "loading"}
                    >
                      Continue
                    </Button>
                    {inviteStatus === "warning" ? (
                      <Button variant="outline" onClick={() => setInviteStatus("idle")}>Try again</Button>
                    ) : null}
                  </div>
                </div>
              ) : (
                <form onSubmit={handleSignUp} className="space-y-4">
                  <label className="block text-sm font-medium">
                    Name
                    <Input
                      value={name}
                      onChange={(event) => setName(event.target.value)}
                      type="text"
                      autoComplete="name"
                      autoFocus
                      className="mt-2"
                    />
                  </label>
                  <label className="block text-sm font-medium">
                    Email
                    <Input
                      value={email}
                      onChange={(event) => setEmail(event.target.value)}
                      type="email"
                      autoComplete="email"
                      required
                      className="mt-2"
                    />
                  </label>
                  <label className="block text-sm font-medium">
                    Password
                    <Input
                      value={password}
                      onChange={(event) => setPassword(event.target.value)}
                      type="password"
                      autoComplete="new-password"
                      required
                      className="mt-2"
                    />
                  </label>
                  {accountError ? (
                    <p
                      className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700"
                      role="alert"
                    >
                      {accountError}
                    </p>
                  ) : null}
                  <div className="flex flex-wrap gap-3">
                    <Button type="submit" disabled={accountLoading} size="lg">
                      {accountLoading ? "Creating account" : "Create account"}
                    </Button>
                    <Link
                      href={`/sign-in?redirect=${encodeURIComponent(`/invite?code=${code}`)}`}
                      className="text-xs text-[var(--atlas-ink-muted)] underline"
                    >
                      Already have an account? Sign in
                    </Link>
                  </div>
                </form>
              )}
            </CardContent>
          </Card>
        ) : null}

        {resolvedStep === "launcher" ? (
          <Card>
            <CardHeader>
              <CardTitle>{playerWebCopy.invite.launcherStepTitle}</CardTitle>
              <CardDescription>
                Open Atlas Launcher to continue directly to your invited pack.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-4 text-sm text-[var(--atlas-ink-muted)]">
                Atlas Launcher opens on your invited pack so you can install and play.
              </div>

              <div className="flex flex-wrap gap-3">
                <Button size="lg" onClick={handleOpenLauncher}>
                  {playerWebCopy.invite.openLauncherCta}
                </Button>
                <a
                  href="/download/app/installer/latest"
                  ref={launcherDownloadRef}
                  className="inline-flex items-center rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
                  rel="noreferrer"
                  target="_blank"
                >
                  {playerWebCopy.invite.downloadLauncherCta}
                </a>
              </div>

              {launcherFallbackVisible ? (
                <div
                  className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700"
                  role="alert"
                >
                  Atlas Launcher did not open automatically. Install it with the download button, then try again.
                </div>
              ) : null}

              <div className="flex flex-wrap gap-3">
                <Button variant="outline" onClick={() => setStepAndUrl("done")}>Continue in launcher</Button>
                {launcherAttempted && deepLink ? (
                  <button
                    type="button"
                    className="text-xs text-[var(--atlas-ink-muted)] underline"
                    onClick={handleOpenLauncher}
                  >
                    Try opening again
                  </button>
                ) : null}
              </div>
            </CardContent>
          </Card>
        ) : null}

        {resolvedStep === "done" ? (
          <Card>
            <CardHeader>
              <CardTitle>{playerWebCopy.invite.doneTitle}</CardTitle>
              <CardDescription>Finish setup in Atlas Launcher.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <ol className="list-decimal space-y-2 pl-4 text-sm text-[var(--atlas-ink-muted)]">
                <li>Open Atlas Launcher.</li>
                <li>Sign in to Atlas Hub if prompted.</li>
                <li>Sign in to Microsoft and complete account linking.</li>
                <li>Install the invited pack and press play.</li>
              </ol>
              <div className="flex flex-wrap gap-3">
                <Button variant="outline" onClick={() => setStepAndUrl("launcher")}>Back</Button>
                <Link href="/dashboard">
                  <Button>Go to dashboard</Button>
                </Link>
              </div>
            </CardContent>
          </Card>
        ) : null}
      </section>
    </div>
  );
}
