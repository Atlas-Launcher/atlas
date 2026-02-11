import { NextResponse } from "next/server";

import { resolveDownloadRedirect } from "@/lib/distribution";

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ downloadId: string }> }
) {
  const { downloadId } = await params;

  if (!downloadId || !downloadId.startsWith("dl_")) {
    return NextResponse.json({ error: "Invalid download ID." }, { status: 400 });
  }

  const location = await resolveDownloadRedirect(downloadId);
  if (!location) {
    return NextResponse.json({ error: "Download not found." }, { status: 404 });
  }

  return NextResponse.redirect(location, {
    status: 302,
    headers: {
      "cache-control": "public, max-age=31536000, immutable",
    },
  });
}
