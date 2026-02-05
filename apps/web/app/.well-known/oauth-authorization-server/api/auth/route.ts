import { NextResponse } from "next/server";

export async function GET(request: Request) {
  const metadataUrl = new URL("/api/auth/.well-known/openid-configuration", request.url);
  const response = await fetch(metadataUrl, {
    method: "GET",
    headers: { Accept: "application/json" },
    cache: "no-store",
  });

  if (!response.ok) {
    return NextResponse.json(
      { error: "Unable to resolve authorization server metadata." },
      { status: response.status }
    );
  }

  const metadata = await response.json();
  return NextResponse.json(metadata, {
    headers: {
      "Cache-Control": "public, max-age=300",
    },
  });
}
