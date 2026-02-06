"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface InviteClientProps {
  code: string | null;
  signedIn: boolean;
}

export default function InviteClient({ code, signedIn }: InviteClientProps) {
  const router = useRouter();
  const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!signedIn || !code) {
      return;
    }

    const acceptInvite = async () => {
      setStatus("loading");
      const response = await fetch("/api/invites/accept", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });
      const data = await response.json();

      if (!response.ok) {
        setError(data?.error ?? "Unable to accept invite.");
        setStatus("error");
        return;
      }

      const nextPackId = data?.packId?.toString();
      if (!nextPackId) {
        setError("Invite accepted, but destination pack was missing.");
        setStatus("error");
        return;
      }

      router.push(`/dashboard/${nextPackId}`);
    };

    acceptInvite().catch(() => {
      setError("Unable to accept invite.");
      setStatus("error");
    });
  }, [code, signedIn, router]);

  if (!code) {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Invite</Badge>
          <CardTitle>Missing invite code</CardTitle>
          <CardDescription>Invite links require a code.</CardDescription>
        </CardHeader>
        <CardContent>
          <Link href="/dashboard">
            <Button>Back to Dashboard</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  if (!signedIn) {
    const redirect = encodeURIComponent(`/invite?code=${code}`);
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Invite</Badge>
          <CardTitle>Join this pack</CardTitle>
          <CardDescription>Sign in or create an account to accept the invite.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-3">
          <Link href={`/sign-in?redirect=${redirect}`}>
            <Button>Sign In</Button>
          </Link>
          <Link href={`/sign-up?redirect=${redirect}`}>
            <Button variant="outline">Create Account</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <Badge variant="secondary">Invite</Badge>
        <CardTitle>Joining pack</CardTitle>
        <CardDescription>We are adding this pack to your account.</CardDescription>
      </CardHeader>
      <CardContent>
        {status === "error" ? (
          <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
            {error ?? "Unable to accept invite."}
          </div>
        ) : (
          <p className="text-sm text-[var(--atlas-ink-muted)]">Processing invite…</p>
        )}
      </CardContent>
    </Card>
  );
}
