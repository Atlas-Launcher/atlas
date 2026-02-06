import { NextResponse } from "next/server";

import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { CurseForgeProxyError, curseForgeGet, pickAllowedParams } from "@/lib/curseforge";

const SEARCH_QUERY_PARAMS = [
  "gameId",
  "classId",
  "categoryId",
  "gameVersion",
  "searchFilter",
  "slug",
  "modLoaderType",
  "sortField",
  "sortOrder",
  "index",
  "pageSize",
] as const;

export async function GET(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const url = new URL(request.url);
  const params = pickAllowedParams(url.searchParams, SEARCH_QUERY_PARAMS);

  try {
    const body = await curseForgeGet("/mods/search", params);
    return NextResponse.json(body);
  } catch (error) {
    if (error instanceof CurseForgeProxyError) {
      return NextResponse.json(error.details, { status: error.status });
    }
    return NextResponse.json({ error: "Unable to query CurseForge." }, { status: 502 });
  }
}
