import { NextResponse } from "next/server";
import { auth } from "@/auth";
import { db } from "@/lib/db";
import { builds, channels } from "@/lib/db/schema";
import { decodeArtifactRef, isStorageProviderEnabled } from "@/lib/storage/harness";

function getApiKey(request: Request) {
  const header = request.headers.get("authorization");
  if (header?.toLowerCase().startsWith("bearer ")) {
    return header.slice(7).trim();
  }
  return request.headers.get("x-api-key")?.trim();
}

export async function POST(request: Request) {
  const apiKey = getApiKey(request);

  if (!apiKey) {
    return NextResponse.json({ error: "Missing API key" }, { status: 401 });
  }

  const verification = await auth.api.verifyApiKey({
    body: { key: apiKey },
  });

  if (!verification?.valid || !verification.key) {
    return NextResponse.json({ error: "Invalid API key" }, { status: 403 });
  }

  const body = await request.json().catch(() => ({}));
  const packIdFromKey = verification.key.metadata?.packId?.toString();
  const keyType = verification.key.metadata?.type?.toString();
  const packId = body?.packId?.toString() ?? packIdFromKey;
  const buildId = body?.buildId?.toString();
  const artifactKey = body?.artifactKey?.toString();
  const version = body?.version?.toString();
  const commitHash = body?.commitHash?.toString();
  const artifactSize = body?.artifactSize ? Number(body.artifactSize) : null;
  const channel = (body?.channel?.toString() ?? "dev") as "dev" | "beta" | "production";

  if (keyType && keyType !== "deploy") {
    return NextResponse.json({ error: "Invalid API key type" }, { status: 403 });
  }

  if (!packId || !packIdFromKey || packId !== packIdFromKey) {
    return NextResponse.json({ error: "Pack mismatch" }, { status: 403 });
  }

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
      artifactKey,
      artifactSize: artifactSize ?? undefined,
    })
    .onConflictDoUpdate({
      target: builds.id,
      set: {
        version,
        commitHash,
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

  return NextResponse.json({ build, channel: channelRow });
}
