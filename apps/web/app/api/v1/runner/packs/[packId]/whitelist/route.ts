import { NextResponse } from "next/server";
import { getAuthenticatedRunnerPackId } from "@/lib/auth/runner-tokens";
import { getWhitelist, getWhitelistByVersion, getWhitelistVersion } from "@/lib/packs/whitelist";

interface RouteParams {
  params: Promise<{ packId: string }>;
}

function etagMatches(ifNoneMatch: string | null, etag: string) {
  if (!ifNoneMatch) return false;

  // Split on commas, trim, drop weak prefix
  const parts = ifNoneMatch.split(",").map((p) => p.trim().replace(/^W\//, ""));
  const cleanEtag = etag.replace(/^W\//, "");

  return parts.includes(cleanEtag) || parts.includes("*");
}

export async function GET(request: Request, { params }: RouteParams) {
  const { packId } = await params;

  const runnerPackId = await getAuthenticatedRunnerPackId(request);
  if (!runnerPackId) return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  if (runnerPackId !== packId) return NextResponse.json({ error: "Forbidden" }, { status: 403 });

  const whitelistVersion = await getWhitelistVersion(packId);
  const etag = `"whitelist-v${whitelistVersion}"`;

  if (etagMatches(request.headers.get("if-none-match"), etag)) {
    return new NextResponse(null, {
      status: 304,
      headers: {
        ETag: etag,
        "Cache-Control": "private, no-store",
      },
    });
  }

    const wl = await getWhitelistByVersion(packId, whitelistVersion, { recomputeIfMissing: true });
    if (!wl) {
        // extremely rare if row changed between calls; just fall back
        const latest = await getWhitelist(packId);
        return NextResponse.json(latest.data, { headers: { ETag: etag } });
    }
    return NextResponse.json(wl.data, { headers: { ETag: etag } });

}
