"use client";

import { useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

export default function CliSigninClient() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const initialUserCode = useMemo(
    () => searchParams.get("user_code")?.toUpperCase() ?? "",
    [searchParams]
  );
  const [userCode, setUserCode] = useState(initialUserCode);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setLoading(true);

    const formattedCode = userCode.trim().replace(/-/g, "").toUpperCase();

    try {
      const response = await authClient.device({
        query: { user_code: formattedCode },
      });

      setLoading(false);

      if (response?.error || !response?.data) {
        setError("Invalid or expired code.");
        return;
      }

      router.push(`/cli/signin/approve?user_code=${formattedCode}`);
    } catch {
      setLoading(false);
      setError("Invalid or expired code.");
    }
  };

  return (
    <div className="min-h-screen bg-transparent px-6 py-16 text-[var(--atlas-ink)]">
      <div className="mx-auto max-w-md">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>CLI Authorization</CardTitle>
            <CardDescription>
              Enter the code from the Atlas CLI to authorize this session.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <form onSubmit={handleSubmit} className="space-y-4">
              <Input
                value={userCode}
                onChange={(event) => setUserCode(event.target.value)}
                placeholder="ABCD-1234"
                maxLength={12}
                className="tracking-[0.3em]"
              />
              {error ? (
                <p className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
                  {error}
                </p>
              ) : null}
              <Button type="submit" disabled={loading} className="w-full">
                {loading ? "Checking" : "Continue"}
              </Button>
            </form>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              Tip: You can paste the code without dashes; we will format it automatically.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
