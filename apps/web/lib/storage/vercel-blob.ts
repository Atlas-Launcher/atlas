import { head } from "@vercel/blob";
import { generateClientTokenFromReadWriteToken } from "@vercel/blob/client";

const VERCEL_BLOB_API_BASE = "https://blob.vercel-storage.com";

interface VercelBlobConfig {
  token: string;
}

interface BlobPutResponse {
  url?: string;
  downloadUrl?: string;
  pathname?: string;
}

function getConfig(): VercelBlobConfig {
  const token = process.env.BLOB_READ_WRITE_TOKEN;
  if (!token) {
    throw new Error("BLOB_READ_WRITE_TOKEN is not configured");
  }

  return { token };
}

function encodeBlobPath(pathname: string) {
  return pathname
    .split("/")
    .filter((segment) => segment.length > 0)
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

function normalizeBlobPathname(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return "";
  }
  try {
    const parsed = new URL(trimmed);
    return parsed.pathname.replace(/^\/+/, "");
  } catch {
    return trimmed.replace(/^\/+/, "");
  }
}

export function isVercelBlobConfigured() {
  return Boolean(process.env.BLOB_READ_WRITE_TOKEN);
}

export async function uploadToVercelBlob({
  pathname,
  body,
  contentType,
}: {
  pathname: string;
  body: ArrayBuffer;
  contentType?: string;
}) {
  const { token } = getConfig();
  const encodedPath = encodeBlobPath(pathname);
  const response = await fetch(`${VERCEL_BLOB_API_BASE}/${encodedPath}`, {
    method: "PUT",
    headers: {
      Authorization: `Bearer ${token}`,
      "x-add-random-suffix": "0",
      "x-content-type": contentType ?? "application/octet-stream",
      "content-type": contentType ?? "application/octet-stream",
    },
    body,
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => "unknown error");
    throw new Error(`Vercel Blob upload failed (${response.status}): ${detail}`);
  }

  const json = (await response.json()) as BlobPutResponse;
  return {
    pathname: json.pathname ?? pathname,
    url: json.url ?? json.downloadUrl ?? null,
  };
}

export async function downloadFromVercelBlob(pathname: string) {
  const { token } = getConfig();
  const encodedPath = encodeBlobPath(pathname);
  const response = await fetch(`${VERCEL_BLOB_API_BASE}/${encodedPath}`, {
    method: "GET",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });

  if (!response.ok) {
    const detail = await response.text().catch(() => "unknown error");
    throw new Error(`Vercel Blob download failed (${response.status}): ${detail}`);
  }

  return response;
}

export async function createVercelBlobDownloadUrl(pathname: string) {
  const { token } = getConfig();
  const normalizedPath = normalizeBlobPathname(pathname);
  if (!normalizedPath) {
    throw new Error("Blob pathname is required");
  }

  const metadata = await head(normalizedPath, { token });
  return metadata.downloadUrl ?? metadata.url;
}

export async function createVercelBlobUploadRequest({
  pathname,
  contentType,
}: {
  pathname: string;
  contentType?: string;
}) {
  const { token } = getConfig();
  const normalizedPath = normalizeBlobPathname(pathname);
  const resolvedContentType = contentType ?? "application/octet-stream";
  if (!normalizedPath) {
    throw new Error("Blob pathname is required");
  }

  const clientToken = await generateClientTokenFromReadWriteToken({
    pathname: normalizedPath,
    token,
    addRandomSuffix: false,
    allowOverwrite: false,
    allowedContentTypes: [resolvedContentType],
    validUntil: Date.now() + 5 * 60 * 1000,
  });

  const encodedPath = encodeBlobPath(normalizedPath);

  return {
    url: `${VERCEL_BLOB_API_BASE}/${encodedPath}`,
    headers: {
      Authorization: `Bearer ${clientToken}`,
      "x-add-random-suffix": "0",
      "x-content-type": resolvedContentType,
      "content-type": resolvedContentType,
    } as Record<string, string>,
  };
}
