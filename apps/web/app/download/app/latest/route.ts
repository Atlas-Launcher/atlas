import { NextResponse } from "next/server";

import { getLatestRelease } from "@/lib/releases";

export async function GET() {
  const release = await getLatestRelease("launcher-v");
  if (!release) {
    return NextResponse.json({ error: "No launcher release found." }, { status: 404 });
  }

  const updateAsset = release.assets.find((asset) => {
    const name = asset.name.toLowerCase();
    return name.includes("latest") && name.endsWith(".json");
  });

  if (!updateAsset) {
    return NextResponse.json(
      { error: "No update manifest found for the latest launcher release." },
      { status: 404 },
    );
  }

  const response = await fetch(updateAsset.browser_download_url, {
    headers: {
      "User-Agent": "atlas-hub-downloads",
    },
    next: { revalidate: 300 },
  });

  if (!response.ok) {
    return NextResponse.json({ error: "Failed to fetch update manifest." }, { status: 502 });
  }

  const body = await response.text();
  return new NextResponse(body, {
    headers: {
      "content-type": "application/json",
      "cache-control": "public, max-age=300",
    },
  });
}
