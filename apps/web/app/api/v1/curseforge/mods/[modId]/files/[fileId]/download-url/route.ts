import { NextResponse } from "next/server";

import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { CurseForgeProxyError, curseForgeGet } from "@/lib/curseforge";

export async function GET(
  request: Request,
  context: { params: Promise<{ modId: string; fileId: string }> }
) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const { modId, fileId } = await context.params;
  if (!modId.trim() || !fileId.trim()) {
    return NextResponse.json(
      { error: "modId and fileId are required" },
      { status: 400 }
    );
  }

  try {
    const body = await curseForgeGet(
      `/mods/${encodeURIComponent(modId)}/files/${encodeURIComponent(fileId)}/download-url`
    );
    return NextResponse.json(body);
  } catch (error) {
    if (error instanceof CurseForgeProxyError) {
      return NextResponse.json(error.details, { status: error.status });
    }
    return NextResponse.json({ error: "Unable to query CurseForge." }, { status: 502 });
  }
}
