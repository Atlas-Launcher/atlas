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
import type { StorageProviderId } from "@/lib/storage/types";

function isDistributionReleaseArtifactKey(key: string) {
  return /^artifacts\/(launcher|cli|runner|runnerd)\//.test(key);
}

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
      const artifactRef =
        providerFromBody && !key.includes("::")
          ? { provider: providerFromBody, key }
          : decodeArtifactRef(key);
      if (!isStorageProviderEnabled(artifactRef.provider)) {
        return NextResponse.json(
          {
            error: `Storage provider '${artifactRef.provider}' is not enabled.`,
          },
          { status: 503 }
        );
      }

      const url = await createDownloadUrlForArtifactRef(artifactRef);
      return NextResponse.json({ url, key, provider: artifactRef.provider });
    } else {
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

      if (isDistributionReleaseArtifactKey(objectKey) && session.user.role !== "admin") {
        return NextResponse.json(
          { error: "Only admins can prepare distribution release uploads." },
          { status: 403 }
        );
      }

      const uploadRequest = await createUploadUrlForProvider({
        provider,
        key: objectKey,
        contentType,
      });

      return NextResponse.json({
        url: uploadRequest.url,
        uploadHeaders: uploadRequest.headers ?? {},
        key: encodeArtifactRef({ provider, key: objectKey }),
        provider,
      });
    }
  } catch (error) {
    console.error("Presign error", error);
    return NextResponse.json(
      { error: "Unable to create presigned URL" },
      { status: 500 }
    );
  }
}
