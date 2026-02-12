"use client";

import { useEffect, useMemo } from "react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { playerWebCopy } from "@/app/_copy/player";

interface LauncherLinkClientProps {
  code: string | null;
  signedIn: boolean;
}

export default function LauncherLinkClient({ code, signedIn }: LauncherLinkClientProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const status = searchParams.get("status");
  const message = searchParams.get("message");

  const shouldClaim = useMemo(
    () => Boolean(signedIn && code && !status),
    [code, signedIn, status]
  );

  useEffect(() => {
    if (!shouldClaim || !code) {
      return;
    }

    const claim = async () => {
      const response = await fetch("/api/v1/launcher/link-sessions/claim", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });
      const data = await response.json();

      const params = new URLSearchParams(searchParams.toString());
      params.set("code", code);

      if (!response.ok) {
        params.set("status", "error");
        params.set("message", data?.error ?? "Unable to claim link session.");
        router.replace(`/link/launcher?${params.toString()}`);
        return;
      }

      params.set("status", "claimed");
      params.delete("message");
      router.replace(`/link/launcher?${params.toString()}`);
    };

    claim().catch(() => {
      const params = new URLSearchParams(searchParams.toString());
      params.set("code", code);
      params.set("status", "error");
      params.set("message", "Unable to claim link session.");
      router.replace(`/link/launcher?${params.toString()}`);
    });
  }, [code, router, searchParams, shouldClaim]);

  if (!code) {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Launcher Link</Badge>
          <CardTitle>Missing link code</CardTitle>
          <CardDescription>Ask the launcher for a new link code.</CardDescription>
        </CardHeader>
        <CardContent>
          <Link href="/sign-in">
            <Button>Back to sign in</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  if (!signedIn) {
    const redirect = encodeURIComponent(`/link/launcher?code=${code}`);
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Launcher Link</Badge>
          <CardTitle>{playerWebCopy.link.title}</CardTitle>
          <CardDescription>Sign in to finish linking your launcher.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-3">
          <Link href={`/sign-in?redirect=${redirect}`}>
            <Button>Sign in</Button>
          </Link>
          <Link href={`/sign-up?redirect=${redirect}`}>
            <Button variant="outline">Create account</Button>
          </Link>
        </CardContent>
      </Card>
    );
  }

  if (status === "error") {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Launcher Link</Badge>
          <CardTitle>Unable to link launcher</CardTitle>
          <CardDescription>{message ?? "Link session failed."}</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-[var(--atlas-ink-muted)]">
            Refresh the page or request a new link code from the launcher.
          </p>
        </CardContent>
      </Card>
    );
  }

  if (status === "claimed") {
    return (
      <Card>
        <CardHeader>
          <Badge variant="secondary">Launcher Link</Badge>
          <CardTitle>Link confirmed</CardTitle>
          <CardDescription>{playerWebCopy.link.success}</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-[var(--atlas-ink-muted)]">
            Your account is ready. The launcher will finish verifying your Minecraft profile.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <Badge variant="secondary">Launcher Link</Badge>
        <CardTitle>Linking your launcher</CardTitle>
        <CardDescription>We are confirming your link session now.</CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-[var(--atlas-ink-muted)]">Claiming link session…</p>
      </CardContent>
    </Card>
  );
}
