import { NextResponse } from "next/server";
import { eq, inArray } from "drizzle-orm";

import { db } from "@/lib/db";
import {
  builds,
  channels,
  packMembers,
  packs,
} from "@/lib/db/schema";
import { allowedChannels } from "@/lib/auth/roles";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { decodeArtifactRef, isStorageProviderEnabled } from "@/lib/storage/harness";

type AccessLevel = "dev" | "beta" | "production" | "all";
type ChannelName = "dev" | "beta" | "production";
type MemberRole = "admin" | "creator" | "player";

interface LauncherRemotePack {
  packId: string;
  packName: string;
  packSlug: string;
  repoUrl: string | null;
  accessLevel: AccessLevel;
  channel: ChannelName;
  buildId: string | null;
  buildVersion: string | null;
  artifactKey: string | null;
  artifactProvider: "r2" | "vercel_blob" | null;
}

function preferredChannel(accessLevel: AccessLevel): ChannelName {
  if (accessLevel === "dev" || accessLevel === "all") {
    return "dev";
  }
  if (accessLevel === "beta") {
    return "beta";
  }
  return "production";
}

function selectChannel(
  accessLevel: AccessLevel,
  role: MemberRole,
  channelRows: Map<
    ChannelName,
    {
      buildId: string | null;
      buildVersion: string | null;
      artifactKey: string | null;
      artifactProvider: "r2" | "vercel_blob" | null;
    }
  >
) {
  const preferred = preferredChannel(accessLevel);
  const allowed = allowedChannels(accessLevel, role) as readonly ChannelName[];

  const preferredRow = channelRows.get(preferred);
  if (preferredRow) {
    return {
      channel: preferred,
      ...preferredRow,
    };
  }

  for (const channel of allowed) {
    const row = channelRows.get(channel);
    if (row) {
      return {
        channel,
        ...row,
      };
    }
  }

  return {
    channel: preferred,
    buildId: null,
    buildVersion: null,
    artifactKey: null,
    artifactProvider: null,
  };
}

export async function GET(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const memberships = await db
    .select({
      packId: packs.id,
      packName: packs.name,
      packSlug: packs.slug,
      repoUrl: packs.repoUrl,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
    })
    .from(packMembers)
    .innerJoin(packs, eq(packMembers.packId, packs.id))
    .where(eq(packMembers.userId, userId));

  if (memberships.length === 0) {
    return NextResponse.json({ packs: [] });
  }

  const packIds = memberships.map((membership) => membership.packId);
  const channelRows = await db
    .select({
      packId: channels.packId,
      name: channels.name,
      buildId: channels.buildId,
      buildVersion: builds.version,
      artifactKey: builds.artifactKey,
    })
    .from(channels)
    .leftJoin(builds, eq(builds.id, channels.buildId))
    .where(inArray(channels.packId, packIds));

  const channelMap = new Map<
    string,
    Map<
      ChannelName,
      {
        buildId: string | null;
        buildVersion: string | null;
        artifactKey: string | null;
        artifactProvider: "r2" | "vercel_blob" | null;
      }
    >
  >();
  for (const row of channelRows) {
    const artifactRef = row.artifactKey ? decodeArtifactRef(row.artifactKey) : null;
    if (artifactRef && !isStorageProviderEnabled(artifactRef.provider)) {
      continue;
    }

    const map = channelMap.get(row.packId) ?? new Map();
    map.set(row.name, {
      buildId: row.buildId ?? null,
      buildVersion: row.buildVersion ?? null,
      artifactKey: artifactRef?.key ?? null,
      artifactProvider: artifactRef?.provider ?? null,
    });
    channelMap.set(row.packId, map);
  }

  const remotePacks: LauncherRemotePack[] = memberships
    .map((membership) => {
      const channelsForPack = channelMap.get(membership.packId) ?? new Map();
      const selected = selectChannel(membership.accessLevel, membership.role, channelsForPack);
      return {
        packId: membership.packId,
        packName: membership.packName,
        packSlug: membership.packSlug,
        repoUrl: membership.repoUrl,
        accessLevel: membership.accessLevel,
        channel: selected.channel,
        buildId: selected.buildId,
        buildVersion: selected.buildVersion,
        artifactKey: selected.artifactKey,
        artifactProvider: selected.artifactProvider,
      };
    })
    .sort((a, b) => a.packName.localeCompare(b.packName));

  return NextResponse.json({ packs: remotePacks });
}
