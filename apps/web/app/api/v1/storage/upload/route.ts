import { NextResponse } from "next/server";

import { isStorageProviderEnabled, uploadViaStorageProvider } from "@/lib/storage/harness";
import { verifyStorageToken } from "@/lib/storage/token";

export async function PUT(request: Request) {
  const token = new URL(request.url).searchParams.get("token");
  if (!token) {
    return NextResponse.json({ error: "Missing upload token." }, { status: 401 });
  }

  let payload: ReturnType<typeof verifyStorageToken>;
  try {
    payload = verifyStorageToken(token, "upload");
  } catch (error) {
    return NextResponse.json(
      { error: `Invalid upload token: ${error instanceof Error ? error.message : "unknown"}` },
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

  const body = await request.arrayBuffer();
  await uploadViaStorageProvider({
    provider: payload.provider,
    key: payload.key,
    body,
    contentType: request.headers.get("content-type") ?? "application/octet-stream",
  });

  return NextResponse.json({ ok: true });
}
