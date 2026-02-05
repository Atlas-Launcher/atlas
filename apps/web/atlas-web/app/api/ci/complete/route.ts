import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { builds, channels, deployTokens } from "@/lib/db/schema";
import { hashDeployToken } from "@/lib/auth/deploy-tokens";

function getToken(request: Request) {
  const header = request.headers.get("authorization");
  if (header?.toLowerCase().startsWith("bearer ")) {
    return header.slice(7).trim();
  }
  return request.headers.get("x-deploy-token")?.trim();
}

export async function POST(request: Request) {
  const token = getToken(request);

  if (!token) {
    return NextResponse.json({ error: "Missing deploy token" }, { status: 401 });
  }

  const tokenHash = hashDeployToken(token);
  const [deployToken] = await db
    .select()
    .from(deployTokens)
    .where(and(eq(deployTokens.tokenHash, tokenHash), eq(deployTokens.active, true)));

  if (!deployToken) {
    return NextResponse.json({ error: "Invalid deploy token" }, { status: 403 });
  }

  const body = await request.json();
  const packId = body?.packId?.toString() ?? deployToken.packId;
  const buildId = body?.buildId?.toString();
  const artifactKey = body?.artifactKey?.toString();
  const version = body?.version?.toString();
  const commitHash = body?.commitHash?.toString();
  const artifactSize = body?.artifactSize ? Number(body.artifactSize) : null;
  const channel = (body?.channel?.toString() ?? "dev") as "dev" | "beta" | "production";

  if (!packId || packId !== deployToken.packId) {
    return NextResponse.json({ error: "Pack mismatch" }, { status: 403 });
  }

  if (!buildId || !artifactKey || !version) {
    return NextResponse.json(
      { error: "buildId, artifactKey, and version are required" },
      { status: 400 }
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
