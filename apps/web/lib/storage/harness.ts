import {
  createPresignedDownloadUrl as createR2DownloadUrl,
  createPresignedUploadUrl as createR2UploadUrl,
  downloadFromR2,
  isR2Configured,
  uploadToR2,
} from "@/lib/storage/r2";
import {
  downloadFromVercelBlob,
  isVercelBlobConfigured,
  uploadToVercelBlob,
} from "@/lib/storage/vercel-blob";
import type { ArtifactRef, StorageProviderId } from "@/lib/storage/types";

const ARTIFACT_REF_DELIMITER = "::";

export function getEnabledStorageProviders(): StorageProviderId[] {
  const enabled: StorageProviderId[] = [];
  if (isR2Configured()) {
    enabled.push("r2");
  }
  if (isVercelBlobConfigured()) {
    enabled.push("vercel_blob");
  }
  return enabled;
}

export function isStorageProviderEnabled(provider: StorageProviderId) {
  return getEnabledStorageProviders().includes(provider);
}

export function getPreferredStorageProvider(): StorageProviderId {
  if (isR2Configured()) {
    return "r2";
  }
  if (isVercelBlobConfigured()) {
    return "vercel_blob";
  }

  throw new Error(
    "No storage provider configured. Set R2_* vars or BLOB_READ_WRITE_TOKEN."
  );
}

export function encodeArtifactRef(ref: ArtifactRef) {
  return `${ref.provider}${ARTIFACT_REF_DELIMITER}${ref.key}`;
}

export function decodeArtifactRef(storedValue: string): ArtifactRef {
  for (const provider of ["r2", "vercel_blob"] as const) {
    const prefix = `${provider}${ARTIFACT_REF_DELIMITER}`;
    if (storedValue.startsWith(prefix)) {
      return {
        provider,
        key: storedValue.slice(prefix.length),
      };
    }
  }

  // Backward compatibility with legacy records that stored raw keys directly.
  // Infer provider from configured backends when possible.
  if (isR2Configured()) {
    return { provider: "r2", key: storedValue };
  }
  if (isVercelBlobConfigured()) {
    return { provider: "vercel_blob", key: storedValue };
  }

  // No provider configured; preserve historical fallback.
  return { provider: "r2", key: storedValue };
}

export async function createUploadUrlForProvider({
  provider,
  key,
  contentType,
}: {
  provider: StorageProviderId;
  key: string;
  contentType?: string;
}) {
  if (!isStorageProviderEnabled(provider)) {
    throw new Error(`Storage provider '${provider}' is not enabled.`);
  }

  if (provider === "r2") {
    return createR2UploadUrl({ key, contentType });
  }

  throw new Error(`Provider '${provider}' does not support direct presigned uploads.`);
}

export async function createDownloadUrlForArtifactRef(ref: ArtifactRef) {
  if (!isStorageProviderEnabled(ref.provider)) {
    throw new Error(`Storage provider '${ref.provider}' is not enabled.`);
  }

  if (ref.provider === "r2") {
    return createR2DownloadUrl({ key: ref.key });
  }

  throw new Error(`Provider '${ref.provider}' does not support direct presigned downloads.`);
}

export async function uploadViaStorageProvider({
  provider,
  key,
  body,
  contentType,
}: {
  provider: StorageProviderId;
  key: string;
  body: ArrayBuffer;
  contentType?: string;
}) {
  if (!isStorageProviderEnabled(provider)) {
    throw new Error(`Storage provider '${provider}' is not enabled.`);
  }

  if (provider === "r2") {
    await uploadToR2({ key, body, contentType });
    return;
  }

  await uploadToVercelBlob({ pathname: key, body, contentType });
}

export async function downloadViaStorageProvider(ref: ArtifactRef) {
  if (!isStorageProviderEnabled(ref.provider)) {
    throw new Error(`Storage provider '${ref.provider}' is not enabled.`);
  }

  if (ref.provider === "r2") {
    return downloadFromR2(ref.key);
  }

  return downloadFromVercelBlob(ref.key);
}
