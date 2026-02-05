"use client";

import { useEffect, useState } from "react";
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

export default function SignInPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect") ?? "/dashboard";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [passkeyAvailable, setPasskeyAvailable] = useState(false);

  useEffect(() => {
    const autoFill = async () => {
      if (
        typeof window !== "undefined" &&
        window.PublicKeyCredential &&
        PublicKeyCredential.isConditionalMediationAvailable
      ) {
        const available = await PublicKeyCredential.isConditionalMediationAvailable();
        if (available) {
          await authClient.signIn.passkey({ autoFill: true });
        }
      }
    };

    autoFill().catch(() => null);
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

    router.push(redirectTo);
  };

  const handlePasskey = async () => {
    setError(null);
    setLoading(true);

    const result = await authClient.signIn.passkey();
    setLoading(false);

    if (result?.error) {
      setError(result.error.message ?? "Passkey sign-in failed.");
      return;
    }

    router.push(redirectTo);
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>Sign in</CardTitle>
            <CardDescription>
              Access your packs, releases, and channel controls.
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
                <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
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
                Sign in with passkey
              </Button>
              <a
                className="block text-center text-xs text-[var(--atlas-ink-muted)] underline"
                href="/sign-up"
              >
                Need an account? Create one
              </a>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
