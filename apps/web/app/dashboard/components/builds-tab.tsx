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
  return (
    <Card>
      <CardHeader>
        <CardTitle>Builds</CardTitle>
        <CardDescription>Immutable builds received from CI and promoted to channels.</CardDescription>
      </CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Version</TableHead>
              <TableHead>Commit</TableHead>
              <TableHead>Deployed</TableHead>
              <TableHead>Live Channels</TableHead>
              <TableHead className="w-[340px]">Actions</TableHead>
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
                    <TableCell className="font-semibold">
                      <div>{build.version}</div>
                      {runtimeMetadata ? (
                        <div className="mt-1 text-xs font-normal text-[var(--atlas-ink-muted)]">
                          {runtimeMetadata}
                        </div>
                      ) : null}
                    </TableCell>
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
                        <div className="flex flex-wrap gap-2">
                          <Button
                            size="sm"
                            variant={build.forceReinstall ? "secondary" : "outline"}
                            disabled={loading}
                            onClick={() =>
                              onToggleForceReinstall(build.id, !Boolean(build.forceReinstall))
                            }
                          >
                            {build.forceReinstall
                              ? "Force Reinstall On"
                              : "Force Reinstall Off"}
                          </Button>
                          {channelOrder.map((channelName) => {
                            const isLive = liveChannels.some(
                              (channel) => channel.name === channelName
                            );

                            return (
                              <Button
                                key={channelName}
                                size="sm"
                                variant={isLive ? "secondary" : "outline"}
                                disabled={loading || isLive}
                                onClick={() => onPromote(channelName, build.id)}
                              >
                                {isLive
                                  ? `${channelLabel(channelName)} live`
                                  : `Set ${channelLabel(channelName)}`}
                              </Button>
                            );
                          })}
                        </div>
                      ) : (
                        <span className="text-xs text-[var(--atlas-ink-muted)]">
                          No promotion permissions
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
