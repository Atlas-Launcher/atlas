import { NextResponse } from "next/server";
import { db } from "@/lib/db";
import { builds, channels } from "@/lib/db/schema";
import { decodeArtifactRef, isStorageProviderEnabled } from "@/lib/storage/harness";
import { resolveCiAuthContext } from "@/lib/ci/auth";
import { emitPackUpdate } from "@/lib/pack-update-events";

export async function POST(request: Request) {
  const body = await request.json().catch(() => ({}));
  let authContext;
  try {
    authContext = await resolveCiAuthContext(request, body?.packId?.toString() ?? null);
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Unauthorized" },
      { status: 403 }
    );
  }
  const packId = authContext.packId;
  const buildId = body?.buildId?.toString();
  const artifactKey = body?.artifactKey?.toString();
  const version = body?.version?.toString();
  const commitHash = body?.commitHash?.toString();
  const commitMessage = normalizeOptionalString(body?.commitMessage);
  const minecraftVersion = normalizeOptionalString(body?.minecraftVersion);
  const modloader = normalizeOptionalString(body?.modloader);
  const modloaderVersion = normalizeOptionalString(body?.modloaderVersion);
  const forceReinstall = body?.forceReinstall === true;
  const artifactSize = body?.artifactSize ? Number(body.artifactSize) : null;
  const channel = (body?.channel?.toString() ?? "dev") as "dev" | "beta" | "production";

  if (!buildId || !artifactKey || !version) {
    return NextResponse.json(
      { error: "buildId, artifactKey, and version are required" },
      { status: 400 }
    );
  }

  const artifactRef = decodeArtifactRef(artifactKey);
  if (!isStorageProviderEnabled(artifactRef.provider)) {
    return NextResponse.json(
      {
        error: `Storage provider '${artifactRef.provider}' is not enabled for this deployment.`,
      },
      { status: 503 }
    );
  }

  const [build] = await db
    .insert(builds)
    .values({
      id: buildId,
      packId,
      version,
      commitHash,
      commitMessage,
      minecraftVersion,
      modloader,
      modloaderVersion,
      forceReinstall,
      artifactKey,
      artifactSize: artifactSize ?? undefined,
    })
    .onConflictDoUpdate({
      target: builds.id,
      set: {
        version,
        commitHash,
        commitMessage,
        minecraftVersion,
        modloader,
        modloaderVersion,
        forceReinstall,
        artifactKey,
        artifactSize: artifactSize ?? undefined,
      },
    })
    .returning();

  const [channelRow] = await db
    .insert(channels)
    .values({
      packId,
      name: channel,
      buildId: build.id,
      updatedAt: new Date(),
    })
    .onConflictDoUpdate({
      target: [channels.packId, channels.name],
      set: { buildId: build.id, updatedAt: new Date() },
    })
    .returning();

  emitPackUpdate({
    packId,
    channel,
    buildId: build.id,
    source: authContext.method,
  });

  return NextResponse.json({ build, channel: channelRow });
}

function normalizeOptionalString(value: unknown): string | null {
  if (value == null) {
    return null;
  }

  const normalized = value.toString().trim();
  return normalized.length ? normalized : null;
}
