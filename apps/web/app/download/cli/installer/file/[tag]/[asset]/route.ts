import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ tag: string; asset: string }> },
) {
  const { tag, asset } = await params;
  const location = new URL(
    `/download/cli/file/${encodeURIComponent(tag)}/${encodeURIComponent(asset)}`,
    request.url,
  );
  return NextResponse.redirect(location, { status: 302 });
}
