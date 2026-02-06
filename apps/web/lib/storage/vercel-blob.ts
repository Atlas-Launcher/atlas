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

export async function blobExistsInVercel(pathname: string) {
  const { token } = getConfig();
  const encodedPath = encodeBlobPath(pathname);
  const response = await fetch(`${VERCEL_BLOB_API_BASE}/${encodedPath}`, {
    method: "HEAD",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });

  if (response.status === 404) {
    return false;
  }

  if (response.ok) {
    return true;
  }

  // Some Blob endpoints may not allow HEAD; do a tiny ranged GET as fallback.
  if (response.status === 405) {
    const probe = await fetch(`${VERCEL_BLOB_API_BASE}/${encodedPath}`, {
      method: "GET",
      headers: {
        Authorization: `Bearer ${token}`,
        Range: "bytes=0-0",
      },
    });
    if (probe.status === 404) {
      return false;
    }
    if (probe.ok || probe.status === 206) {
      return true;
    }
    const detail = await probe.text().catch(() => "unknown error");
    throw new Error(`Vercel Blob existence check failed (${probe.status}): ${detail}`);
  }

  const detail = await response.text().catch(() => "unknown error");
  throw new Error(`Vercel Blob existence check failed (${response.status}): ${detail}`);
}
