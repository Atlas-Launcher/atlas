import { NextResponse } from "next/server";
import { and, eq, inArray } from "drizzle-orm";

import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { allowedChannels } from "@/lib/auth/roles";
import { db } from "@/lib/db";
import { builds, channels, packMembers } from "@/lib/db/schema";
import {
  createDownloadUrlForArtifactRef,
  decodeArtifactRef,
  isStorageProviderEnabled,
} from "@/lib/storage/harness";
import { createStorageToken } from "@/lib/storage/token";

type AccessLevel = "dev" | "beta" | "production" | "all";
type ChannelName = "dev" | "beta" | "production";
type MemberRole = "admin" | "creator" | "player";

interface ChannelBuildRow {
  buildId: string | null;
  buildVersion: string | null;
  artifactKey: string | null;
  minecraftVersion: string | null;
  modloader: string | null;
  modloaderVersion: string | null;
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

function parseChannelName(value: string | null): ChannelName | null {
  if (value === "dev" || value === "beta" || value === "production") {
    return value;
  }
  return null;
}

function selectChannelWithBuild(
  rowsByChannel: Map<ChannelName, ChannelBuildRow>,
  allowed: readonly ChannelName[],
  accessLevel: AccessLevel,
  requested: ChannelName | null
) {
  const ordered = new Set<ChannelName>();
  if (requested && allowed.includes(requested)) {
    ordered.add(requested);
  }
  ordered.add(preferredChannel(accessLevel));
  for (const name of allowed) {
    ordered.add(name);
  }

  for (const name of ordered) {
    const row = rowsByChannel.get(name);
    if (row?.buildId && row.artifactKey) {
      return { channel: name, row };
    }
  }

  return null;
}

export async function GET(
  request: Request,
  context: { params: Promise<{ packId: string }> }
) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const params = await context.params;
  const packId = params.packId?.trim();
  if (!packId) {
    return NextResponse.json({ error: "Missing pack id." }, { status: 400 });
  }

  const [membership] = await db
    .select({
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
    })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  if (!membership) {
    return NextResponse.json({ error: "Pack not found." }, { status: 404 });
  }

  const allowed = allowedChannels(
    membership.accessLevel as AccessLevel,
    membership.role as MemberRole
  ) as readonly ChannelName[];
  const requestedChannel = parseChannelName(new URL(request.url).searchParams.get("channel"));

  const rows = await db
    .select({
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
    .where(and(eq(channels.packId, packId), inArray(channels.name, [...allowed])));

  const rowsByChannel = new Map<ChannelName, ChannelBuildRow>();
  for (const row of rows) {
    rowsByChannel.set(row.name, {
      buildId: row.buildId ?? null,
      buildVersion: row.buildVersion ?? null,
      artifactKey: row.artifactKey ?? null,
      minecraftVersion: row.minecraftVersion ?? null,
      modloader: row.modloader ?? null,
      modloaderVersion: row.modloaderVersion ?? null,
    });
  }

  const selected = selectChannelWithBuild(
    rowsByChannel,
    allowed,
    membership.accessLevel as AccessLevel,
    requestedChannel
  );
  if (!selected) {
    return NextResponse.json(
      { error: "No build is available for this pack." },
      { status: 404 }
    );
  }

  const artifactKey = selected.row.artifactKey;
  if (!artifactKey) {
    return NextResponse.json(
      { error: "Selected build does not have an artifact." },
      { status: 404 }
    );
  }

  const artifactRef = decodeArtifactRef(artifactKey);
  if (!isStorageProviderEnabled(artifactRef.provider)) {
    return NextResponse.json(
      { error: `Storage provider '${artifactRef.provider}' is not enabled.` },
      { status: 503 }
    );
  }

  let downloadUrl: string;
  if (artifactRef.provider === "r2") {
    downloadUrl = await createDownloadUrlForArtifactRef(artifactRef);
  } else {
    const token = createStorageToken({
      action: "download",
      provider: artifactRef.provider,
      key: artifactRef.key,
      expiresInSeconds: 900,
    });
    const origin = new URL(request.url).origin;
    downloadUrl = `${origin}/api/storage/download?token=${encodeURIComponent(token)}`;
  }

  return NextResponse.json({
    packId,
    channel: selected.channel,
    buildId: selected.row.buildId,
    buildVersion: selected.row.buildVersion,
    artifactKey: artifactRef.key,
    artifactProvider: artifactRef.provider,
    downloadUrl,
    minecraftVersion: selected.row.minecraftVersion,
    modloader: selected.row.modloader,
    modloaderVersion: selected.row.modloaderVersion,
  });
}
