import { NextResponse } from "next/server";
import { and, eq } from "drizzle-orm";

import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";
import { db } from "@/lib/db";
import { builds, channels } from "@/lib/db/schema";

interface RouteParams {
  params: Promise<{ packId: string }>;
}

function etagMatches(ifNoneMatch: string | null, etag: string) {
  if (!ifNoneMatch) return false;
  const parts = ifNoneMatch.split(",").map((p) => p.trim().replace(/^W\//, ""));
  const clean = etag.replace(/^W\//, "");
  return parts.includes(clean) || parts.includes("*");
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;

  const runnerPackId = await getAuthenticatedRunnerPackId(request);
  if (!runnerPackId) return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  if (runnerPackId !== packId) return NextResponse.json({ error: "Forbidden" }, { status: 403 });

  const url = new URL(request.url);
  const rawChannel = url.searchParams.get("channel") || "production";
  const allowedChannels = ["production", "dev", "beta"] as const;
  type ChannelName = (typeof allowedChannels)[number];
  const channel: ChannelName = allowedChannels.includes(rawChannel as ChannelName)
    ? (rawChannel as ChannelName)
    : "production";

  const [row] = await db
    .select({
      buildId: channels.buildId,
      buildVersion: builds.version,
      minecraftVersion: builds.minecraftVersion,
      modloader: builds.modloader,
      modloaderVersion: builds.modloaderVersion,
      createdAt: builds.createdAt,
    })
    .from(channels)
    .leftJoin(builds, eq(builds.id, channels.buildId))
    .where(and(eq(channels.packId, packId), eq(channels.name, channel)))
    .limit(1);

  if (!row?.buildId || row.buildVersion == null) {
    return NextResponse.json({ error: "No build found for channel" }, { status: 404 });
  }

  const metadata = {
    packId,
    channel,
    buildId: row.buildId,
    version: row.buildVersion,
    minecraftVersion: row.minecraftVersion,
    modloader: row.modloader,
    modloaderVersion: row.modloaderVersion,
    updatedAt: row.createdAt?.toISOString(),
  };

  // Prefer an etag that changes whenever the build changes
  const etag = `"pack-${packId}-${channel}-${row.buildId}-${row.buildVersion}"`;

  if (etagMatches(request.headers.get("if-none-match"), etag)) {
    return new NextResponse(null, {
      status: 304,
      headers: {
        ETag: etag,
        "Cache-Control": "private, no-store",
      },
    });
  }

  return NextResponse.json(metadata, {
    headers: {
      ETag: etag,
      "Cache-Control": "private, no-store",
    },
  });
}
