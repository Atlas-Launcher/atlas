import { NextResponse } from "next/server";

import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { CurseForgeProxyError, curseForgeGet, pickAllowedParams } from "@/lib/curseforge";

const FILES_QUERY_PARAMS = [
  "gameVersion",
  "modLoaderType",
  "index",
  "pageSize",
] as const;

export async function GET(
  request: Request,
  context: { params: Promise<{ modId: string }> }
) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { modId } = await context.params;
  if (!modId.trim()) {
    return NextResponse.json({ error: "modId is required" }, { status: 400 });
  }

  const url = new URL(request.url);
  const params = pickAllowedParams(url.searchParams, FILES_QUERY_PARAMS);

  try {
    const body = await curseForgeGet(`/mods/${encodeURIComponent(modId)}/files`, params);
    return NextResponse.json(body);
  } catch (error) {
    if (error instanceof CurseForgeProxyError) {
      return NextResponse.json(error.details, { status: error.status });
    }
    return NextResponse.json({ error: "Unable to query CurseForge." }, { status: 502 });
  }
}
