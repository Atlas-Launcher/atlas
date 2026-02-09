"use client";

import { useEffect, useMemo, useState } from "react";
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

type InviteStep = "account" | "microsoft" | "download" | "done";

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

type InviteStatus = "idle" | "loading" | "accepted" | "warning" | "error";

const MICROSOFT_SCOPES = [
  "openid",
  "profile",
  "email",
  "XboxLive.signin",
  "offline_access",
];

const steps: { id: InviteStep; label: string; detail: string }[] = [
  {
    id: "account",
    label: "Create account",
    detail: "Set up your Atlas login.",
  },
  {
    id: "microsoft",
    label: "Link Microsoft",
    detail: "Required for launcher access.",
  },
  {
    id: "download",
    label: "Download launcher",
    detail: "Grab the installer.",
  },
  {
    id: "done",
    label: "Finish",
    detail: "Sign in on the launcher.",
  },
];

function isInviteStep(value: string | null): value is InviteStep {
  return value === "account" || value === "microsoft" || value === "download" || value === "done";
}

export default function InviteClient({ code, signedIn }: InviteClientProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [preview, setPreview] = useState<InvitePreview | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [signedInState, setSignedInState] = useState(signedIn);
  const [inviteStatus, setInviteStatus] = useState<InviteStatus>("idle");
  const [inviteError, setInviteError] = useState<string | null>(null);
  const [msLinked, setMsLinked] = useState(false);
  const [msLoading, setMsLoading] = useState(false);
  const [msError, setMsError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [accountLoading, setAccountLoading] = useState(false);
  const [accountError, setAccountError] = useState<string | null>(null);

  const stepParam = searchParams.get("step");
  const initialStep = useMemo<InviteStep>(() => {
    if (isInviteStep(stepParam)) return stepParam;
    if (!signedIn) return "account";
    if (signedIn && !msLinked) return "microsoft";
    return "account";
  }, [msLinked, signedIn, stepParam]);
  const [manualStep, setManualStep] = useState<InviteStep>(initialStep);
  const resolvedStep = useMemo<InviteStep>(() => {
    if (isInviteStep(stepParam)) return stepParam;
    if (signedInState && msLinked && manualStep === "microsoft") return "account";
    return manualStep;
  }, [manualStep, msLinked, signedInState, stepParam]);

  useEffect(() => {
    if (!code) {
      return;
    }
    const loadPreview = async () => {
      setPreviewError(null);
      const response = await fetch(`/api/invites/preview?code=${encodeURIComponent(code)}`);
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
    if (!signedInState) {
      return;
    }

    const loadAccounts = async () => {
      const response = await fetch("/api/auth/list-accounts");
      const data = await response.json();
      if (!response.ok) {
        return;
      }
      const linked = (data ?? []).some(
        (account: { providerId?: string }) => account.providerId === "microsoft"
      );
      setMsLinked(linked);
    };

    loadAccounts().catch(() => null);
  }, [signedInState]);

  useEffect(() => {
    if (!signedInState || !code || inviteStatus !== "idle") {
      return;
    }

    const acceptInvite = async () => {
      setInviteStatus("loading");
      setInviteError(null);
      const response = await fetch("/api/invites/accept", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });
      const data = await response.json();

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

      setInviteStatus("accepted");
    };

    acceptInvite().catch(() => {
      setInviteStatus("warning");
      setInviteError("Unable to accept invite.");
    });
  }, [code, inviteStatus, signedInState]);

  const setStepAndUrl = (nextStep: InviteStep) => {
    if (!code) {
      setManualStep(nextStep);
      return;
    }
    const params = new URLSearchParams(searchParams.toString());
    params.set("code", code);
    params.set("step", nextStep);
    setManualStep(nextStep);
    router.replace(`/invite?${params.toString()}`);
  };

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

  const handleLinkMicrosoft = async () => {
    if (!code || msLoading) {
      return;
    }
    setMsError(null);
    setMsLoading(true);
    const callbackURL = new URL(`/invite?code=${code}&step=microsoft`, window.location.origin).toString();
    const response = await fetch("/api/auth/link-social", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        provider: "microsoft",
        callbackURL,
        scopes: MICROSOFT_SCOPES,
        disableRedirect: true,
      }),
    });
    const data = await response.json();
    setMsLoading(false);

    if (!response.ok) {
      setMsError(data?.error ?? "Unable to link Microsoft.");
      return;
    }

    if (data?.url) {
      window.location.href = data.url;
    } else {
      setMsError("Unable to start Microsoft linking.");
    }
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
          <h2 className="mt-4 text-3xl font-semibold">Welcome to Atlas.</h2>
          <p className="mt-3 text-sm text-[var(--atlas-ink-muted)]">
            {preview?.pack?.name
              ? `You're joining ${preview.pack.name}. We'll set up your account, link Microsoft, and get you the launcher.`
              : "We will set up your account, link Microsoft, and get you the launcher."}
          </p>
          {preview?.pack?.name ? (
            <div className="mt-5 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
              <p className="text-sm font-semibold text-[var(--atlas-ink)]">{preview.pack.name}</p>
              <p className="mt-1">
                {preview.creator?.name
                  ? `Created by ${preview.creator.name}`
                  : "Pack creator"}
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
                      <p className="text-sm font-semibold text-[var(--atlas-ink)]">
                        {item.label}
                      </p>
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
              <CardTitle>Create your Atlas account</CardTitle>
              <CardDescription>We will auto-accept the invite as soon as you are in.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {accountComplete ? (
                <div className="space-y-4">
                  <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                    Account ready. We are connecting your invite now.
                  </div>
                  {inviteStatus === "loading" ? (
                    <p className="text-xs text-[var(--atlas-ink-muted)]">Accepting invite…</p>
                  ) : null}
                  {inviteStatus === "accepted" ? (
                    <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                      Invite accepted. You are in the pack.
                    </div>
                  ) : null}
                  {inviteStatus === "warning" ? (
                    <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
                      {inviteError ?? "Invite acceptance needs attention."}
                    </div>
                  ) : null}
                  <Button
                    onClick={() => setStepAndUrl(msLinked ? "download" : "microsoft")}
                    disabled={inviteStatus === "loading"}
                  >
                    Continue
                  </Button>
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
                    <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
                      {accountError}
                    </p>
                  ) : null}
                  <div className="flex flex-wrap gap-3">
                    <Button type="submit" disabled={accountLoading} size="lg">
                      {accountLoading ? "Creating" : "Create Account"}
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

        {resolvedStep === "microsoft" ? (
          <Card>
            <CardHeader>
              <CardTitle>Link your Microsoft account</CardTitle>
              <CardDescription>Required for the launcher to sync your Minecraft profile.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {msLinked ? (
                <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                  Microsoft connected. You are ready to download the launcher.
                </div>
              ) : (
                <div className="space-y-3">
                  <p className="text-sm text-[var(--atlas-ink-muted)]">
                    We will open a Microsoft sign-in window so you can grant access.
                  </p>
                  {msError ? (
                    <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
                      {msError}
                    </div>
                  ) : null}
                  <Button onClick={handleLinkMicrosoft} disabled={msLoading} size="lg">
                    {msLoading ? "Opening" : "Link Microsoft"}
                  </Button>
                </div>
              )}
              <div className="flex flex-wrap gap-3">
                <Button
                  variant="outline"
                  onClick={() => setStepAndUrl("account")}
                >
                  Back
                </Button>
                <Button
                  onClick={() => setStepAndUrl("download")}
                  disabled={!msLinked}
                >
                  Continue
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : null}

        {resolvedStep === "download" ? (
          <Card>
            <CardHeader>
              <CardTitle>Download the Atlas Launcher</CardTitle>
              <CardDescription>Your installer starts right away.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-4 text-sm text-[var(--atlas-ink-muted)]">
                The launcher will prompt you to sign in with Microsoft and sync your packs.
              </div>
              <div className="flex flex-wrap gap-3">
                <a
                  href="/download/app/installer/latest"
                  className="rounded-full bg-[var(--atlas-ink)] px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-cream)] shadow-[0_12px_30px_rgba(16,20,24,0.25)] transition hover:-translate-y-0.5"
                  rel="noreferrer"
                  target="_blank"
                >
                  Download Launcher
                </a>
                <Link
                  href="/download/app"
                  className="rounded-full border border-[var(--atlas-ink)]/20 bg-white/70 px-6 py-3 text-sm font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink)] transition hover:-translate-y-0.5"
                >
                  View all downloads
                </Link>
              </div>
              <div className="flex flex-wrap gap-3">
                <Button variant="outline" onClick={() => setStepAndUrl("microsoft")}>Back</Button>
                <Button onClick={() => setStepAndUrl("done")}>Continue</Button>
              </div>
            </CardContent>
          </Card>
        ) : null}

        {resolvedStep === "done" ? (
          <Card>
            <CardHeader>
              <CardTitle>You are ready to play</CardTitle>
              <CardDescription>Finish by signing in on the launcher.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <ol className="list-decimal space-y-2 pl-4 text-sm text-[var(--atlas-ink-muted)]">
                <li>Open the Atlas Launcher you just downloaded.</li>
                <li>Sign in with your Microsoft account.</li>
                <li>Select your pack and launch.</li>
              </ol>
              <div className="flex flex-wrap gap-3">
                <Button variant="outline" onClick={() => setStepAndUrl("download")}>Back</Button>
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
