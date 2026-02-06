import { NextResponse } from "next/server";

import { downloadViaStorageProvider, isStorageProviderEnabled } from "@/lib/storage/harness";
import { verifyStorageToken } from "@/lib/storage/token";

export async function GET(request: Request) {
  const token = new URL(request.url).searchParams.get("token");
  if (!token) {
    return NextResponse.json({ error: "Missing download token." }, { status: 401 });
  }

  let payload: ReturnType<typeof verifyStorageToken>;
  try {
    payload = verifyStorageToken(token, "download");
  } catch (error) {
    return NextResponse.json(
      { error: `Invalid download token: ${error instanceof Error ? error.message : "unknown"}` },
      { status: 401 }
    );
  }

  if (!isStorageProviderEnabled(payload.provider)) {
    return NextResponse.json(
      {
        error: `Storage provider '${payload.provider}' is not enabled.`,
      },
      { status: 503 }
    );
  }

  const result = await downloadViaStorageProvider({
    provider: payload.provider,
    key: payload.key,
  });

  if (result instanceof Response) {
    return new Response(result.body, {
      status: 200,
      headers: {
        "content-type":
          result.headers.get("content-type") ?? "application/octet-stream",
        "cache-control": result.headers.get("cache-control") ?? "private, max-age=300",
      },
    });
  }

  return new Response(result.stream, {
    status: 200,
    headers: {
      "content-type": result.contentType,
      ...(result.contentLength ? { "content-length": String(result.contentLength) } : {}),
      "cache-control": "private, max-age=300",
    },
  });
}
