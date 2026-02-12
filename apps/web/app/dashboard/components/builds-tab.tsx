"use client";

import { useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { Build, Channel } from "@/app/dashboard/types";
import { toGithubCommitUrl } from "@/lib/github";

interface BuildsTabProps {
  channels: Channel[];
  builds: Build[];
  repoUrl?: string | null;
  canPromoteBuilds: boolean;
  onPromote: (channel: Channel["name"], buildId: string) => void;
  onToggleForceReinstall: (buildId: string, forceReinstall: boolean) => void;
  loading: boolean;
}

const channelOrder: Channel["name"][] = ["dev", "beta", "production"];
const channelLabel = (name: Channel["name"]) => name[0].toUpperCase() + name.slice(1);

export default function BuildsTab({
  channels,
  builds,
  repoUrl,
  canPromoteBuilds,
  onPromote,
  onToggleForceReinstall,
  loading,
}: BuildsTabProps) {
  const [targetByBuild, setTargetByBuild] = useState<Record<string, Channel["name"]>>({});

  return (
    <Card>
      <CardHeader>
        <CardTitle>Builds</CardTitle>
        <CardDescription>Review builds and promote channels.</CardDescription>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Commit</TableHead>
              <TableHead>Version</TableHead>
              <TableHead>Deployed</TableHead>
              <TableHead>Live Channels</TableHead>
              <TableHead className="w-[320px]">Promotion</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {builds.length ? (
              builds.map((build) => {
                const liveChannels = channels.filter((channel) => channel.buildId === build.id);
                const commitUrl = toGithubCommitUrl(repoUrl, build.commitHash);
                const runtimeMetadata = formatRuntimeMetadata(build);

                return (
                  <TableRow key={build.id}>
                    <TableCell>
                      {build.commitHash ? (
                        <div className="space-y-1">
                          {commitUrl ? (
                            <a
                              href={commitUrl}
                              target="_blank"
                              rel="noreferrer"
                              className="font-mono underline-offset-2 hover:underline"
                              title={build.commitHash}
                            >
                              {shortHash(build.commitHash)}
                            </a>
                          ) : (
                            <span className="font-mono" title={build.commitHash}>
                              {shortHash(build.commitHash)}
                            </span>
                          )}
                          {build.commitMessage ? (
                            <p className="text-xs text-[var(--atlas-ink-muted)]">
                              {build.commitMessage}
                            </p>
                          ) : null}
                        </div>
                      ) : (
                        "-"
                      )}
                    </TableCell>
                    <TableCell className="font-semibold">
                      {runtimeMetadata ? <div>{runtimeMetadata}</div> : <div>-</div>}
                      {displayVersionLabel(build) ? (
                        <div className="mt-1 text-xs font-normal text-[var(--atlas-ink-muted)]">
                          {displayVersionLabel(build)}
                        </div>
                      ) : null}
                    </TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {formatDeployDate(build.createdAt)}
                    </TableCell>
                    <TableCell>
                      <div className="flex flex-wrap gap-2">
                        {liveChannels.length ? (
                          liveChannels.map((channel) => (
                            <Badge key={channel.id} variant="secondary">
                              {channel.name}
                            </Badge>
                          ))
                        ) : (
                          <span className="text-xs text-[var(--atlas-ink-muted)]">Not promoted</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      {canPromoteBuilds ? (
                        <div className="flex flex-wrap items-center gap-2">
                          <select
                            className="h-8 rounded-md border border-input bg-background px-2 text-xs"
                            value={targetByBuild[build.id] ?? "production"}
                            onChange={(event) =>
                              setTargetByBuild((prev) => ({
                                ...prev,
                                [build.id]: event.target.value as Channel["name"],
                              }))
                            }
                            disabled={loading}
                          >
                            {channelOrder.map((channelName) => (
                              <option key={channelName} value={channelName}>
                                {channelLabel(channelName)}
                              </option>
                            ))}
                          </select>
                          <Button
                            size="sm"
                            disabled={loading}
                            onClick={() =>
                              onPromote(targetByBuild[build.id] ?? "production", build.id)
                            }
                          >
                            Promote
                          </Button>
                          <Button
                            size="sm"
                            variant={build.forceReinstall ? "secondary" : "outline"}
                            disabled={loading}
                            onClick={() =>
                              onToggleForceReinstall(build.id, !Boolean(build.forceReinstall))
                            }
                          >
                            {build.forceReinstall ? "Force reinstall: on" : "Force reinstall: off"}
                          </Button>
                        </div>
                      ) : (
                        <span className="text-xs text-[var(--atlas-ink-muted)]">
                          You do not have permission to promote builds.
                        </span>
                      )}
                    </TableCell>
                  </TableRow>
                );
              })
            ) : (
              <TableRow>
                <TableCell colSpan={5} className="text-sm text-[var(--atlas-ink-muted)]">
                  No builds yet.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}

function shortHash(value: string) {
  return value.length > 12 ? value.slice(0, 12) : value;
}

function formatRuntimeMetadata(build: Build): string | null {
  const mc = build.minecraftVersion?.trim();
  const loader = build.modloader?.trim();
  const loaderVersion = build.modloaderVersion?.trim();

  if (!mc && !loader) {
    return null;
  }

  const loaderText = loader
    ? loaderVersion
      ? `${loader} ${loaderVersion}`
      : loader
    : null;

  if (mc && loaderText) {
    return `MC ${mc} · ${loaderText}`;
  }
  if (mc) {
    return `MC ${mc}`;
  }
  return loaderText;
}

function displayVersionLabel(build: Build): string | null {
  const raw = build.version?.trim();
  if (!raw || isCommitLike(raw)) {
    return null;
  }
  return raw;
}

function isCommitLike(value: string): boolean {
  return /^[0-9a-f]{7,40}$/i.test(value);
}

function formatDeployDate(value?: string): string {
  if (!value) {
    return "-";
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return "-";
  }
  return parsed.toLocaleString();
}
