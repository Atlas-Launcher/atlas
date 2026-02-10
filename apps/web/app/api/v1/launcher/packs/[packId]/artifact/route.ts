import { NextResponse } from "next/server";
import { and, eq, gt, inArray, lte } from "drizzle-orm";

import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";
import { allowedChannels } from "@/lib/auth/roles";
import { db } from "@/lib/db";
import { builds, channels, packMembers } from "@/lib/db/schema";
import {
  createDownloadUrlForArtifactRef,
  decodeArtifactRef,
  isStorageProviderEnabled,
} from "@/lib/storage/harness";

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
  forceReinstall: boolean;
  createdAt: Date | null;
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

function parseChannelName(value: string | null): ChannelName | null {
  if (value === "dev" || value === "beta" || value === "production") {
    return value;
  }
  return null;
}

function buildChannelOrder(
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
  return [...ordered];
}

function normalizeArtifactKey(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) {
    return "";
  }

  try {
    const parsed = new URL(trimmed);
    const pathname = parsed.pathname.replace(/^\/+/, "");
    return pathname || "";
  } catch {
    return trimmed.replace(/^\/+/, "");
  }
}

export async function GET(
  request: Request,
  context: { params: Promise<{ packId: string }> }
) {
  const requestUrl = new URL(request.url);
  const userId = await getAuthenticatedUserId(request);
  let runnerPackId: string | null = null;
  if (!userId) {
    runnerPackId = await getAuthenticatedRunnerPackId(request);
    if (!runnerPackId) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
  }

  const params = await context.params;
  const packId = params.packId?.trim();
  if (!packId) {
    return NextResponse.json({ error: "Missing pack id." }, { status: 400 });
  }

  if (runnerPackId) {
    if (runnerPackId !== packId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }
  }

  const membership = userId
    ? (
      await db
        .select({
          role: packMembers.role,
          accessLevel: packMembers.accessLevel,
        })
        .from(packMembers)
        .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
        .limit(1)
    )[0]
    : null;

  if (!runnerPackId && !membership) {
    return NextResponse.json({ error: "Pack not found." }, { status: 404 });
  }

  const requestedChannel = parseChannelName(requestUrl.searchParams.get("channel"));
  const currentBuildId = requestUrl.searchParams.get("currentBuildId")?.trim() || null;

  let allowed: readonly ChannelName[];
  if (runnerPackId) {
    allowed = ["dev", "beta", "production"] as const;
  } else {
    if (!membership) {
      return NextResponse.json({ error: "Pack not found." }, { status: 404 });
    }
    allowed = allowedChannels(
      membership.accessLevel as AccessLevel,
      membership.role as MemberRole
    ) as readonly ChannelName[];
  }

  const rows = await db
    .select({
      name: channels.name,
      buildId: channels.buildId,
      buildVersion: builds.version,
      artifactKey: builds.artifactKey,
      minecraftVersion: builds.minecraftVersion,
      modloader: builds.modloader,
      modloaderVersion: builds.modloaderVersion,
      forceReinstall: builds.forceReinstall,
      createdAt: builds.createdAt,
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
      forceReinstall: row.forceReinstall ?? false,
      createdAt: row.createdAt ?? null,
    });
  }

  const orderedChannels = runnerPackId
    ? buildChannelOrder(allowed, "all", requestedChannel)
    : buildChannelOrder(
        allowed,
        membership!.accessLevel as AccessLevel,
        requestedChannel
      );
  for (const channelName of orderedChannels) {
    const row = rowsByChannel.get(channelName);
    if (!row?.buildId || !row.artifactKey) {
      continue;
    }

    const artifactRef = decodeArtifactRef(row.artifactKey);
    if (!isStorageProviderEnabled(artifactRef.provider)) {
      continue;
    }

    const resolvedArtifactKey = normalizeArtifactKey(artifactRef.key);
    if (!resolvedArtifactKey) {
      continue;
    }

    const downloadUrl = await createDownloadUrlForArtifactRef({
      provider: artifactRef.provider,
      key: resolvedArtifactKey,
    });
    const requiresFullReinstall = await shouldRequireFullReinstall({
      packId,
      targetBuildId: row.buildId,
      targetBuildCreatedAt: row.createdAt,
      currentBuildId,
    });

    return NextResponse.json({
      packId,
      channel: channelName,
      buildId: row.buildId,
      buildVersion: row.buildVersion,
      artifactKey: resolvedArtifactKey,
      artifactProvider: artifactRef.provider,
      downloadUrl,
      minecraftVersion: row.minecraftVersion,
      modloader: row.modloader,
      modloaderVersion: row.modloaderVersion,
      forceReinstall: row.forceReinstall,
      requiresFullReinstall,
    });
  }

  return NextResponse.json(
    { error: "No downloadable build is available for this pack." },
    { status: 404 }
  );
}

async function shouldRequireFullReinstall({
  packId,
  targetBuildId,
  targetBuildCreatedAt,
  currentBuildId,
}: {
  packId: string;
  targetBuildId: string;
  targetBuildCreatedAt: Date | null;
  currentBuildId: string | null;
}): Promise<boolean> {
  if (!currentBuildId || currentBuildId === targetBuildId || !targetBuildCreatedAt) {
    return false;
  }

  const [currentBuild] = await db
    .select({
      createdAt: builds.createdAt,
    })
    .from(builds)
    .where(and(eq(builds.packId, packId), eq(builds.id, currentBuildId)))
    .limit(1);
  if (!currentBuild?.createdAt) {
    return false;
  }
  if (currentBuild.createdAt >= targetBuildCreatedAt) {
    return false;
  }

  const [flagged] = await db
    .select({ id: builds.id })
    .from(builds)
    .where(
      and(
        eq(builds.packId, packId),
        eq(builds.forceReinstall, true),
        gt(builds.createdAt, currentBuild.createdAt),
        lte(builds.createdAt, targetBuildCreatedAt)
      )
    )
    .limit(1);
  return Boolean(flagged);
}
