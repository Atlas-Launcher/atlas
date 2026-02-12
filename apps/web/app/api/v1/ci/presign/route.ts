import { NextResponse } from "next/server";
import crypto from "crypto";
import {
  createUploadUrlForProvider,
  encodeArtifactRef,
  getPreferredStorageProvider,
} from "@/lib/storage/harness";
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

    const uploadRequest = await createUploadUrlForProvider({
      provider,
      key: objectKey,
      contentType: "application/octet-stream",
    });

    return NextResponse.json({
      buildId,
      artifactKey,
      uploadUrl: uploadRequest.url,
      uploadHeaders: uploadRequest.headers ?? {},
      artifactProvider: provider,
    });
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Unable to prepare upload URL." },
      { status: 503 }
    );
  }
}
