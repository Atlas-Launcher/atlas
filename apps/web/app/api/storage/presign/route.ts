import { NextResponse } from "next/server";

import { auth } from "@/auth";
import { hasRole } from "@/lib/auth/roles";
import {
  createDownloadUrlForArtifactRef,
  createUploadUrlForProvider,
  decodeArtifactRef,
  encodeArtifactRef,
  getPreferredStorageProvider,
  isStorageProviderEnabled,
} from "@/lib/storage/harness";
import { createStorageToken } from "@/lib/storage/token";
import type { StorageProviderId } from "@/lib/storage/types";

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  try {
    const body = await request.json();
    const key = body?.key?.toString();
    const contentType = body?.contentType?.toString();
    const action = body?.action?.toString() ?? "upload";
    const providerFromBody = body?.provider?.toString() as StorageProviderId | undefined;

    if (!key) {
      return NextResponse.json({ error: "Key is required" }, { status: 400 });
    }

    if (action === "download") {
      const artifactRef = decodeArtifactRef(key);
      if (!isStorageProviderEnabled(artifactRef.provider)) {
        return NextResponse.json(
          {
            error: `Storage provider '${artifactRef.provider}' is not enabled.`,
          },
          { status: 503 }
        );
      }

      if (artifactRef.provider === "r2") {
        const url = await createDownloadUrlForArtifactRef(artifactRef);
        return NextResponse.json({ url, key, provider: artifactRef.provider });
      }

      const token = createStorageToken({
        action: "download",
        provider: artifactRef.provider,
        key: artifactRef.key,
        expiresInSeconds: 900,
      });
      const origin = new URL(request.url).origin;
      const url = `${origin}/api/storage/download?token=${encodeURIComponent(token)}`;
      return NextResponse.json({ url, key, provider: artifactRef.provider });
    }

    let provider: StorageProviderId;
    let objectKey: string;

    if (key.includes("::")) {
      const artifactRef = decodeArtifactRef(key);
      provider = artifactRef.provider;
      objectKey = artifactRef.key;
    } else {
      provider = providerFromBody ?? getPreferredStorageProvider();
      objectKey = key;
    }

    if (!isStorageProviderEnabled(provider)) {
      return NextResponse.json(
        {
          error: `Storage provider '${provider}' is not enabled.`,
        },
        { status: 503 }
      );
    }

    let url: string;
    if (provider === "r2") {
      url = await createUploadUrlForProvider({
        provider,
        key: objectKey,
        contentType,
      });
    } else {
      const token = createStorageToken({
        action: "upload",
        provider,
        key: objectKey,
        expiresInSeconds: 900,
      });
      const origin = new URL(request.url).origin;
      url = `${origin}/api/storage/upload?token=${encodeURIComponent(token)}`;
    }

    return NextResponse.json({
      url,
      key: encodeArtifactRef({ provider, key: objectKey }),
      provider,
    });
  } catch (error) {
    console.error("Presign error", error);
    return NextResponse.json(
      { error: "Unable to create presigned URL" },
      { status: 500 }
    );
  }
}
