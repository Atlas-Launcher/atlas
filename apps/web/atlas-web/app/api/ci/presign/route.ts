import { NextResponse } from "next/server";
import crypto from "crypto";
import { and, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { deployTokens } from "@/lib/db/schema";
import { createPresignedUploadUrl } from "@/lib/storage/r2";
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

  if (!packId || packId !== deployToken.packId) {
    return NextResponse.json({ error: "Pack mismatch" }, { status: 403 });
  }

  const buildId = crypto.randomUUID();
  const artifactKey = `packs/${packId}/builds/${buildId}.bin`;
  const uploadUrl = await createPresignedUploadUrl({
    key: artifactKey,
    contentType: "application/octet-stream",
  });

  return NextResponse.json({ buildId, artifactKey, uploadUrl });
}
