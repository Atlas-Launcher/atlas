import { NextResponse } from "next/server";
import { and, eq, gt, inArray } from "drizzle-orm";

import { db } from "@/lib/db";
import {
  builds,
  channels,
  oauthAccessTokens,
  packMembers,
  packs,
} from "@/lib/db/schema";
import { allowedChannels } from "@/lib/auth/roles";

type AccessLevel = "dev" | "beta" | "production";
type ChannelName = "dev" | "beta" | "production";

interface LauncherRemotePack {
  packId: string;
  packName: string;
  packSlug: string;
  accessLevel: AccessLevel;
  channel: ChannelName;
  buildId: string | null;
  buildVersion: string | null;
  artifactKey: string | null;
}

function parseBearerToken(request: Request): string | null {
  const header = request.headers.get("authorization")?.trim();
  if (!header) {
    return null;
  }
  const [scheme, token] = header.split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }
  return token.trim() || null;
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
  channelRows: Map<ChannelName, { buildId: string | null; buildVersion: string | null; artifactKey: string | null }>
) {
  const preferred = preferredChannel(accessLevel);
  const allowed = allowedChannels(accessLevel) as readonly ChannelName[];

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
  };
}

export async function GET(request: Request) {
  const launcherClientId =
    process.env.ATLAS_OIDC_LAUNCHER_CLIENT_ID ?? "atlas-launcher";
  const bearer = parseBearerToken(request);
  if (!bearer) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [token] = await db
    .select({
      userId: oauthAccessTokens.userId,
      expiresAt: oauthAccessTokens.accessTokenExpiresAt,
    })
    .from(oauthAccessTokens)
    .where(
      and(
        eq(oauthAccessTokens.accessToken, bearer),
        eq(oauthAccessTokens.clientId, launcherClientId),
        gt(oauthAccessTokens.accessTokenExpiresAt, new Date())
      )
    )
    .limit(1);

  if (!token?.userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const memberships = await db
    .select({
      packId: packs.id,
      packName: packs.name,
      packSlug: packs.slug,
      accessLevel: packMembers.accessLevel,
    })
    .from(packMembers)
    .innerJoin(packs, eq(packMembers.packId, packs.id))
    .where(eq(packMembers.userId, token.userId));

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
    Map<ChannelName, { buildId: string | null; buildVersion: string | null; artifactKey: string | null }>
  >();
  for (const row of channelRows) {
    const map = channelMap.get(row.packId) ?? new Map();
    map.set(row.name, {
      buildId: row.buildId ?? null,
      buildVersion: row.buildVersion ?? null,
      artifactKey: row.artifactKey ?? null,
    });
    channelMap.set(row.packId, map);
  }

  const remotePacks: LauncherRemotePack[] = memberships
    .map((membership) => {
      const channelsForPack = channelMap.get(membership.packId) ?? new Map();
      const selected = selectChannel(membership.accessLevel, channelsForPack);
      return {
        packId: membership.packId,
        packName: membership.packName,
        packSlug: membership.packSlug,
        accessLevel: membership.accessLevel,
        channel: selected.channel,
        buildId: selected.buildId,
        buildVersion: selected.buildVersion,
        artifactKey: selected.artifactKey,
      };
    })
    .sort((a, b) => a.packName.localeCompare(b.packName));

  return NextResponse.json({ packs: remotePacks });
}
