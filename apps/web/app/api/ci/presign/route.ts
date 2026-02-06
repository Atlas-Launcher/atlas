import { NextResponse } from "next/server";
import crypto from "crypto";
import {
  createUploadUrlForProvider,
  encodeArtifactRef,
  getPreferredStorageProvider,
} from "@/lib/storage/harness";
import { createStorageToken } from "@/lib/storage/token";
import { resolveCiAuthContext } from "@/lib/ci/auth";

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

  try {
    const buildId = crypto.randomUUID();
    const objectKey = `packs/${packId}/builds/${buildId}.atlas`;
    const provider = getPreferredStorageProvider();
    const artifactKey = encodeArtifactRef({ provider, key: objectKey });

    let uploadUrl: string;
    if (provider === "r2") {
      uploadUrl = await createUploadUrlForProvider({
        provider,
        key: objectKey,
        contentType: "application/octet-stream",
      });
    } else {
      const token = createStorageToken({
        action: "upload",
        provider,
        key: objectKey,
        expiresInSeconds: 900,
      });
      const origin = new URL(request.url).origin;
      uploadUrl = `${origin}/api/storage/upload?token=${encodeURIComponent(token)}`;
    }

    return NextResponse.json({
      buildId,
      artifactKey,
      uploadUrl,
      artifactProvider: provider,
    });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Unable to prepare upload URL." },
      { status: 503 }
    );
  }
}
