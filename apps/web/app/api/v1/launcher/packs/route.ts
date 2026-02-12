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
  minecraftVersion: string | null;
  modloader: string | null;
  modloaderVersion: string | null;
}

function rolePriority(role: MemberRole): number {
  if (role === "admin") {
    return 3;
  }
  if (role === "creator") {
    return 2;
  }
  return 1;
}

function accessPriority(accessLevel: AccessLevel): number {
  if (accessLevel === "all") {
    return 4;
  }
  if (accessLevel === "dev") {
    return 3;
  }
  if (accessLevel === "beta") {
    return 2;
  }
  return 1;
}

function preferredChannel(accessLevel: AccessLevel): ChannelName {
  if (accessLevel === "dev") {
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
      minecraftVersion: string | null;
      modloader: string | null;
      modloaderVersion: string | null;
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
    minecraftVersion: null,
    modloader: null,
    modloaderVersion: null,
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

  // Guard against duplicate membership rows for the same user/pack.
  // Keep the strongest role/access pair so launcher receives one pack entry.
  const membershipsByPackId = new Map<
    string,
    (typeof memberships)[number]
  >();
  for (const membership of memberships) {
    const existing = membershipsByPackId.get(membership.packId);
    if (!existing) {
      membershipsByPackId.set(membership.packId, membership);
      continue;
    }
    if (
      rolePriority(membership.role) > rolePriority(existing.role) ||
      (rolePriority(membership.role) === rolePriority(existing.role) &&
        accessPriority(membership.accessLevel) > accessPriority(existing.accessLevel))
    ) {
      membershipsByPackId.set(membership.packId, membership);
    }
  }
  const uniqueMemberships = [...membershipsByPackId.values()];

  const packIds = uniqueMemberships.map((membership) => membership.packId);
  const channelRows = await db
    .select({
      packId: channels.packId,
      name: channels.name,
      buildId: channels.buildId,
      buildVersion: builds.version,
      artifactKey: builds.artifactKey,
      minecraftVersion: builds.minecraftVersion,
      modloader: builds.modloader,
      modloaderVersion: builds.modloaderVersion,
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
        minecraftVersion: string | null;
        modloader: string | null;
        modloaderVersion: string | null;
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
      minecraftVersion: row.minecraftVersion ?? null,
      modloader: row.modloader ?? null,
      modloaderVersion: row.modloaderVersion ?? null,
    });
    channelMap.set(row.packId, map);
  }

  const remotePacks: LauncherRemotePack[] = uniqueMemberships
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
        minecraftVersion: selected.minecraftVersion,
        modloader: selected.modloader,
        modloaderVersion: selected.modloaderVersion,
      };
    })
    .sort((a, b) => a.packName.localeCompare(b.packName));

  return NextResponse.json({ packs: remotePacks }, {
    headers: {
      "Cache-Control": "private, max-age=300", // Cache for 5 minutes
    },
  });
}
