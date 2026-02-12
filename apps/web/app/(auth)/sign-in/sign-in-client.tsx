"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";

function isPasskeyAbortError(error: unknown) {
  if (!error) {
    return false;
  }

  if (typeof error === "string") {
    return /abort|cancelled|canceled|notallowed/i.test(error);
  }

  if (error instanceof Error) {
    return (
      error.name === "AbortError" ||
      /abort|cancelled|canceled|notallowed/i.test(error.message)
    );
  }

  if (typeof error === "object") {
    const candidate = error as { name?: unknown; message?: unknown };
    const name = typeof candidate.name === "string" ? candidate.name : "";
    const message = typeof candidate.message === "string" ? candidate.message : "";
    return (
      name === "AbortError" || /abort|cancelled|canceled|notallowed/i.test(message)
    );
  }

  return false;
}

export default function SignInClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect") ?? "/dashboard";
  const oidcQuery = searchParams.toString();
  const shouldResumeOidcAuthorization = Boolean(
    searchParams.get("response_type") &&
      searchParams.get("client_id") &&
      searchParams.get("redirect_uri")
  );
  const signUpHref =
    shouldResumeOidcAuthorization && oidcQuery
      ? `/sign-up?${oidcQuery}`
      : `/sign-up?redirect=${encodeURIComponent(redirectTo)}`;
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [passkeyAvailable, setPasskeyAvailable] = useState(false);
  const passkeyCeremonyInFlight = useRef(false);

  useEffect(() => {
    const autoFill = async () => {
      if (
        typeof window !== "undefined" &&
        window.PublicKeyCredential &&
        PublicKeyCredential.isConditionalMediationAvailable &&
        !passkeyCeremonyInFlight.current
      ) {
        const available = await PublicKeyCredential.isConditionalMediationAvailable();
        if (available) {
          passkeyCeremonyInFlight.current = true;
          try {
            const result = await authClient.signIn.passkey({ autoFill: true });
            if (result?.error && !isPasskeyAbortError(result.error)) {
              setError(result.error.message ?? "Passkey sign-in failed.");
            }
          } finally {
            passkeyCeremonyInFlight.current = false;
          }
        }
      }
    };

    autoFill().catch((autoFillError) => {
      if (!isPasskeyAbortError(autoFillError)) {
        setError("Passkey sign-in failed.");
      }
      passkeyCeremonyInFlight.current = false;
    });
  }, []);

  useEffect(() => {
    setPasskeyAvailable(typeof window !== "undefined" && "PublicKeyCredential" in window);
  }, []);

  const handleCredentials = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setLoading(true);

    const result = await authClient.signIn.email({
      email,
      password,
    });

    setLoading(false);

    if (result?.error) {
      setError(result.error.message ?? "Invalid email or password.");
      return;
    }

    if (shouldResumeOidcAuthorization && oidcQuery) {
      window.location.href = `/api/auth/oauth2/authorize?${oidcQuery}`;
      return;
    }

    router.push(redirectTo);
  };

  const handlePasskey = async () => {
    if (passkeyCeremonyInFlight.current) {
      return;
    }

    setError(null);
    setLoading(true);
    passkeyCeremonyInFlight.current = true;

    try {
      const result = await authClient.signIn.passkey();

      if (result?.error) {
        if (!isPasskeyAbortError(result.error)) {
          setError(result.error.message ?? "Passkey sign-in failed.");
        }
        return;
      }

      if (shouldResumeOidcAuthorization && oidcQuery) {
        window.location.href = `/api/auth/oauth2/authorize?${oidcQuery}`;
        return;
      }

      router.push(redirectTo);
    } catch (passkeyError) {
      if (!isPasskeyAbortError(passkeyError)) {
        setError("Passkey sign-in failed.");
      }
    } finally {
      setLoading(false);
      passkeyCeremonyInFlight.current = false;
    }
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>Sign in</CardTitle>
            <CardDescription>
              Continue onboarding and open Atlas Launcher.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <form onSubmit={handleCredentials} className="space-y-4">
              <label className="block text-sm font-medium">
                Email
                <Input
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  type="email"
                  autoComplete="username webauthn"
                  autoFocus
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
                  autoComplete="current-password webauthn"
                  required
                  className="mt-2"
                />
              </label>

              {error ? (
                <p
                  className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700"
                  role="alert"
                  aria-live="polite"
                >
                  {error}
                </p>
              ) : null}

              <Button type="submit" disabled={loading} size="lg" className="w-full">
                {loading ? "Signing in" : "Sign in"}
              </Button>
            </form>

            <div className="space-y-3">
              <Button
                type="button"
                variant="outline"
                size="lg"
                className="w-full"
                disabled={!passkeyAvailable || loading}
                onClick={handlePasskey}
              >
                Continue with passkey
              </Button>
              <a
                className="block text-center text-xs text-[var(--atlas-ink-muted)] underline"
                href={signUpHref}
              >
                Need an account? Create Atlas account
              </a>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
