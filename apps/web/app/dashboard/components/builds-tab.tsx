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

interface BuildsTabProps {
  channels: Channel[];
  builds: Build[];
  canPromoteBuilds: boolean;
  onPromote: (channel: Channel["name"], buildId: string) => void;
  loading: boolean;
}

const channelOrder: Channel["name"][] = ["dev", "beta", "production"];
const channelLabel = (name: Channel["name"]) => name[0].toUpperCase() + name.slice(1);

export default function BuildsTab({
  channels,
  builds,
  canPromoteBuilds,
  onPromote,
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
              <TableHead>Artifact</TableHead>
              <TableHead>Live Channels</TableHead>
              <TableHead className="w-[340px]">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {builds.length ? (
              builds.map((build) => {
                const liveChannels = channels.filter((channel) => channel.buildId === build.id);

                return (
                  <TableRow key={build.id}>
                    <TableCell className="font-semibold">{build.version}</TableCell>
                    <TableCell>{build.commitHash ?? "-"}</TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {build.artifactKey}
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
