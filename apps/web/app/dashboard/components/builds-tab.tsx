"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { Build, Channel } from "@/app/dashboard/types";

interface BuildsTabProps {
  channels: Channel[];
  builds: Build[];
  canPromoteBuilds: boolean;
  promotionChannel: string;
  promotionBuild: string;
  onPromotionChannelChange: (value: string) => void;
  onPromotionBuildChange: (value: string) => void;
  onPromote: () => void;
  loading: boolean;
}

export default function BuildsTab({
  channels,
  builds,
  canPromoteBuilds,
  promotionChannel,
  promotionBuild,
  onPromotionChannelChange,
  onPromotionBuildChange,
  onPromote,
  loading,
}: BuildsTabProps) {
  return (
    <>
      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Release Channels</CardTitle>
            <CardDescription>Immutable builds, mutable pointers.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {channels.length ? (
              channels.map((channel) => (
                <div
                  key={channel.id}
                  className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3"
                >
                  <div className="flex items-center justify-between text-sm font-semibold">
                    <span>{channel.name.toUpperCase()}</span>
                    <Badge variant="secondary">Live</Badge>
                  </div>
                  <p className="mt-2 text-xs text-[var(--atlas-ink-muted)]">
                    {channel.buildVersion ?? "No build"}{" "}
                    {channel.buildCommit ? `(${channel.buildCommit})` : ""}
                  </p>
                </div>
              ))
            ) : (
              <p className="text-sm text-[var(--atlas-ink-muted)]">Select a pack to view channels.</p>
            )}
          </CardContent>
        </Card>

        {canPromoteBuilds ? (
          <Card>
            <CardHeader>
              <CardTitle>Promote Build</CardTitle>
              <CardDescription>Move a channel pointer to a new build.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Channel
                <select
                  value={promotionChannel}
                  onChange={(event) => onPromotionChannelChange(event.target.value)}
                  className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                >
                  <option value="dev">Dev</option>
                  <option value="beta">Beta</option>
                  <option value="production">Production</option>
                </select>
              </label>
              <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Build
                <select
                  value={promotionBuild}
                  onChange={(event) => onPromotionBuildChange(event.target.value)}
                  className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                >
                  <option value="">Select build</option>
                  {builds.map((build) => (
                    <option key={build.id} value={build.id}>
                      {build.version} {build.commitHash ? `(${build.commitHash})` : ""}
                    </option>
                  ))}
                </select>
              </label>
              <Button onClick={onPromote} disabled={loading}>
                Promote
              </Button>
            </CardContent>
          </Card>
        ) : null}
      </div>

      <Separator className="my-6" />

      <Card>
        <CardHeader>
          <CardTitle>Build Ledger</CardTitle>
          <CardDescription>Immutable builds received from CI.</CardDescription>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Version</TableHead>
                <TableHead>Commit</TableHead>
                <TableHead>Artifact</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {builds.length ? (
                builds.map((build) => (
                  <TableRow key={build.id}>
                    <TableCell className="font-semibold">{build.version}</TableCell>
                    <TableCell>{build.commitHash ?? "-"}</TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {build.artifactKey}
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={3} className="text-sm text-[var(--atlas-ink-muted)]">
                    Select a pack to view builds.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </>
  );
}
