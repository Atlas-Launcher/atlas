"use client";

import { useMemo, useState } from "react";
import { useSearchParams } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export default function CliApproveClient() {
  const searchParams = useSearchParams();
  const userCode = useMemo(() => {
    const code = searchParams.get("user_code");
    return code ? code.toUpperCase() : "";
  }, [searchParams]);

  const { data: session } = authClient.useSession();
  const [status, setStatus] = useState<"idle" | "approved" | "denied">("idle");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleApprove = async () => {
    setLoading(true);
    setError(null);
    const result = await authClient.device.approve({ userCode });
    setLoading(false);

    if (result?.error) {
      setError(result.error.error_description ?? "Unable to approve CLI sign-in.");
      return;
    }

    setStatus("approved");
  };

  const handleDeny = async () => {
    setLoading(true);
    setError(null);
    const result = await authClient.device.deny({ userCode });
    setLoading(false);

    if (result?.error) {
      setError(result.error.error_description ?? "Unable to deny CLI sign-in.");
      return;
    }

    setStatus("denied");
  };

  return (
    <div className="min-h-screen bg-transparent px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>Approve CLI Sign-in</CardTitle>
            <CardDescription>
              Confirm access for the CLI using code {userCode || "-"}.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {!userCode ? (
              <p className="text-sm text-[var(--atlas-ink-muted)]">
                Missing device code. Return to the CLI and request a new one.
              </p>
            ) : !session ? (
              <div className="space-y-3">
                <p className="text-sm text-[var(--atlas-ink-muted)]">
                  Sign in to approve this CLI authorization.
                </p>
                <Button asChild>
                  <a href={`/sign-in?redirect=/cli/signin/approve?user_code=${userCode}`}>Sign in</a>
                </Button>
              </div>
            ) : (
              <div className="space-y-3">
                {status === "approved" ? (
                  <div className="rounded-2xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-700">
                    CLI approved. Return to your terminal to finish sign-in.
                  </div>
                ) : null}
                {status === "denied" ? (
                  <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
                    CLI denied. You can restart the flow in the CLI.
                  </div>
                ) : null}
                {error ? (
                  <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
                    {error}
                  </div>
                ) : null}

                <div className="flex flex-wrap gap-3">
                  <Button onClick={handleApprove} disabled={loading || status === "approved"}>
                    Approve
                  </Button>
                  <Button
                    variant="outline"
                    onClick={handleDeny}
                    disabled={loading || status === "denied"}
                  >
                    Deny
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
