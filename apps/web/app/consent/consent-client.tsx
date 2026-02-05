"use client";

import { useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function ConsentClient() {
  const searchParams = useSearchParams();
  const { data: session, isPending } = authClient.useSession();
  const [loading, setLoading] = useState<"approve" | "deny" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const consentCode = searchParams.get("consent_code");
  const clientId = searchParams.get("client_id");
  const scopes = useMemo(() => {
    const scope = searchParams.get("scope");
    if (!scope) {
      return [];
    }
    return scope
      .split(" ")
      .map((item) => item.trim())
      .filter(Boolean);
  }, [searchParams]);
  const redirectBack = useMemo(() => {
    const query = searchParams.toString();
    return query ? `/consent?${query}` : "/consent";
  }, [searchParams]);

  const handleConsent = async (accept: boolean) => {
    if (!consentCode) {
      setError("Missing consent code.");
      return;
    }

    setError(null);
    setLoading(accept ? "approve" : "deny");

    try {
      const response = await fetch("/api/auth/oauth2/consent", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          accept,
          consent_code: consentCode,
        }),
      });
      const data = await response.json().catch(() => null);

      if (!response.ok) {
        setLoading(null);
        setError(data?.error_description ?? "Unable to process consent.");
        return;
      }

      if (typeof data?.redirectURI !== "string") {
        setLoading(null);
        setError("Missing redirect target from authorization server.");
        return;
      }

      window.location.href = data.redirectURI;
    } catch {
      setLoading(null);
      setError("Unable to process consent.");
    }
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>Authorize Application</CardTitle>
            <CardDescription>Review requested access before continuing.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {!consentCode ? (
              <p className="text-sm text-[var(--atlas-ink-muted)]">
                Missing consent request. Start this flow from your app again.
              </p>
            ) : isPending ? (
              <p className="text-sm text-[var(--atlas-ink-muted)]">Checking your session…</p>
            ) : !session ? (
              <div className="space-y-3">
                <p className="text-sm text-[var(--atlas-ink-muted)]">
                  Sign in to continue authorization.
                </p>
                <Button asChild className="w-full">
                  <a href={`/sign-in?redirect=${encodeURIComponent(redirectBack)}`}>Sign in</a>
                </Button>
              </div>
            ) : (
              <div className="space-y-4">
                <div className="rounded-2xl border border-[var(--atlas-border)] bg-[var(--atlas-cream)] px-4 py-3 text-sm">
                  <p className="font-semibold text-[var(--atlas-ink)]">
                    Client: {clientId ?? "Unknown client"}
                  </p>
                  <p className="mt-1 text-xs text-[var(--atlas-ink-muted)]">
                    Requested scopes: {scopes.length > 0 ? scopes.join(", ") : "None"}
                  </p>
                </div>

                {error ? (
                  <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
                    {error}
                  </p>
                ) : null}

                <div className="flex flex-wrap gap-3">
                  <Button
                    onClick={() => handleConsent(true)}
                    disabled={loading !== null}
                    className="flex-1"
                  >
                    {loading === "approve" ? "Approving" : "Approve"}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => handleConsent(false)}
                    disabled={loading !== null}
                    className="flex-1"
                  >
                    {loading === "deny" ? "Denying" : "Deny"}
                  </Button>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
