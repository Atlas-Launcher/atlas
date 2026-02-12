"use client";

import { useState } from "react";
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

export default function SignUpClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect") ?? "/dashboard";
  const oidcQuery = searchParams.toString();
  const shouldResumeOidcAuthorization = Boolean(
    searchParams.get("response_type") &&
      searchParams.get("client_id") &&
      searchParams.get("redirect_uri")
  );
  const signInHref =
    shouldResumeOidcAuthorization && oidcQuery
      ? `/sign-in?${oidcQuery}`
      : `/sign-in?redirect=${encodeURIComponent(redirectTo)}`;
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setLoading(true);

    const resolvedName = name.trim() || email.split("@")[0];
    const result = await authClient.signUp.email({
      email,
      password,
      name: resolvedName,
      role: "player",
    });

    setLoading(false);

    if (result?.error) {
      setError(result.error.message ?? "Unable to create account.");
      return;
    }

    if (shouldResumeOidcAuthorization && oidcQuery) {
      window.location.href = `/api/auth/oauth2/authorize?${oidcQuery}`;
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
            <CardTitle>Create Atlas account</CardTitle>
            <CardDescription>
              Create your account and continue onboarding in Atlas Launcher.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <form onSubmit={handleSubmit} className="space-y-4">
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
                  autoComplete="new-password"
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
                {loading ? "Creating" : "Create Atlas account"}
              </Button>
            </form>

            <a
              className="block text-center text-xs text-[var(--atlas-ink-muted)] underline"
              href={signInHref}
            >
              Already have an Atlas account? Sign in
            </a>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
