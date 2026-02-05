"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

interface AccountTabProps {
  onAddPasskey: () => void;
  githubLinked: boolean;
  githubLoading: boolean;
  githubError: string | null;
  onLinkGithub: () => void;
  onUnlinkGithub: () => void;
  focus: "github" | null;
  nextPath: string | null;
}

export default function AccountTab({
  onAddPasskey,
  githubLinked,
  githubLoading,
  githubError,
  onLinkGithub,
  onUnlinkGithub,
  focus,
  nextPath,
}: AccountTabProps) {
  return (
    <div className="space-y-6">
      {focus === "github" ? (
        <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
          Connect a GitHub account to create new repositories. After linking, you&apos;ll
          be returned to {nextPath ?? "/dashboard/create"}.
        </div>
      ) : null}

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Passkeys</CardTitle>
            <CardDescription>
              Register a hardware-backed passkey for quick sign-in.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button onClick={onAddPasskey}>Add Passkey</Button>
          </CardContent>
        </Card>

        <Card className={focus === "github" ? "ring-2 ring-amber-200" : ""}>
          <CardHeader>
            <CardTitle>GitHub</CardTitle>
            <CardDescription>
              Link GitHub to create repositories and pull org ownership.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-semibold">
                  {githubLinked ? "Connected" : "Not connected"}
                </p>
                <p className="text-xs text-[var(--atlas-ink-muted)]">
                  {githubLinked
                    ? "GitHub is linked to your account."
                    : "Link GitHub to continue creating pack repositories."}
                </p>
              </div>
              {githubLinked ? (
                <Button
                  variant="outline"
                  onClick={onUnlinkGithub}
                  disabled={githubLoading}
                >
                  Disconnect
                </Button>
              ) : (
                <Button onClick={onLinkGithub} disabled={githubLoading}>
                  Connect GitHub
                </Button>
              )}
            </div>
            {githubError ? (
              <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
                {githubError}
              </div>
            ) : null}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
