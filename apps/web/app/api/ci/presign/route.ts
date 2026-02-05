import { NextResponse } from "next/server";
import crypto from "crypto";
import { auth } from "@/auth";
import { createPresignedUploadUrl } from "@/lib/storage/r2";

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

  if (keyType && keyType !== "deploy") {
    return NextResponse.json({ error: "Invalid API key type" }, { status: 403 });
  }

  if (!packId || !packIdFromKey || packId !== packIdFromKey) {
    return NextResponse.json({ error: "Pack mismatch" }, { status: 403 });
  }

  const buildId = crypto.randomUUID();
  const artifactKey = `packs/${packId}/builds/${buildId}.atlas`;
  const uploadUrl = await createPresignedUploadUrl({
    key: artifactKey,
    contentType: "application/octet-stream",
  });

  return NextResponse.json({ buildId, artifactKey, uploadUrl });
}
