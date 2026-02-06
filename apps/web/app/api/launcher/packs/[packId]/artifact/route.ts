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
import { blobExistsInVercel } from "@/lib/storage/vercel-blob";
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

function parseBlobKeyFromUrl(value: string): string | null {
  try {
    const parsed = new URL(value);
    const pathname = parsed.pathname.replace(/^\/+/, "");
    return pathname || null;
  } catch {
    return null;
  }
}

function resolveArtifactKeyCandidates({
  packId,
  buildId,
  artifactKey,
}: {
  packId: string;
  buildId: string;
  artifactKey: string;
}) {
  const trimmed = artifactKey.trim();
  if (!trimmed) {
    return [];
  }

  const candidates = new Set<string>();
  const normalizedKey = trimmed.replace(/^\/+/, "");
  const keyFromUrl = parseBlobKeyFromUrl(trimmed);

  candidates.add(normalizedKey);
  if (keyFromUrl) {
    candidates.add(keyFromUrl);
  }

  if (!normalizedKey.includes("/")) {
    candidates.add(`packs/${packId}/builds/${normalizedKey}`);
  }

  const canonicalFileName = `${buildId}.atlas`;
  candidates.add(canonicalFileName);
  candidates.add(`packs/${packId}/builds/${canonicalFileName}`);

  return [...candidates];
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

  const orderedChannels = buildChannelOrder(
    allowed,
    membership.accessLevel as AccessLevel,
    requestedChannel
  );
  const origin = new URL(request.url).origin;

  for (const channelName of orderedChannels) {
    const row = rowsByChannel.get(channelName);
    if (!row?.buildId || !row.artifactKey) {
      continue;
    }

    const artifactRef = decodeArtifactRef(row.artifactKey);
    if (!isStorageProviderEnabled(artifactRef.provider)) {
      continue;
    }

    let resolvedArtifactKey = artifactRef.key;
    if (artifactRef.provider === "vercel_blob") {
      try {
        const candidates = resolveArtifactKeyCandidates({
          packId,
          buildId: row.buildId,
          artifactKey: artifactRef.key,
        });
        let matched: string | null = null;
        let hadProbeError = false;
        for (const candidate of candidates) {
          try {
            if (await blobExistsInVercel(candidate)) {
              matched = candidate;
              break;
            }
          } catch (error) {
            hadProbeError = true;
            console.warn(
              "Launcher artifact probe failed; will continue optimistically",
              JSON.stringify({
                packId,
                channelName,
                candidate,
                error: error instanceof Error ? error.message : String(error),
              })
            );
          }
        }
        if (!matched) {
          if (hadProbeError) {
            resolvedArtifactKey = candidates[0] ?? artifactRef.key;
          } else {
            console.warn(
              "Launcher artifact pointer missing in Vercel Blob",
              JSON.stringify({ packId, channelName, key: artifactRef.key, buildId: row.buildId })
            );
            continue;
          }
        } else {
          resolvedArtifactKey = matched;
        }
      } catch (error) {
        console.warn(
          "Launcher artifact resolution failed; falling back to stored key",
          JSON.stringify({
            packId,
            channelName,
            key: artifactRef.key,
            buildId: row.buildId,
            error: error instanceof Error ? error.message : String(error),
          })
        );
        resolvedArtifactKey = artifactRef.key;
      }

      if (!resolvedArtifactKey.trim()) {
        console.warn(
          "Launcher artifact key resolved empty",
          JSON.stringify({
            packId,
            channelName,
            key: artifactRef.key,
            buildId: row.buildId,
          })
        );
        continue;
      }
    }

    let downloadUrl: string;
    if (artifactRef.provider === "r2") {
      downloadUrl = await createDownloadUrlForArtifactRef(artifactRef);
    } else {
      const token = createStorageToken({
        action: "download",
        provider: artifactRef.provider,
        key: resolvedArtifactKey,
        expiresInSeconds: 900,
      });
      downloadUrl = `${origin}/api/storage/download?token=${encodeURIComponent(token)}`;
    }

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
    });
  }

  return NextResponse.json(
    { error: "No downloadable build is available for this pack." },
    { status: 404 }
  );
}
