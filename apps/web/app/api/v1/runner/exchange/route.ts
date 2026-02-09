import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { runnerServiceTokens } from "@/lib/db/schema";
import {
  createRunnerAccessToken,
  resolveRunnerServiceToken,
} from "@/lib/auth/runner-tokens";

export const runtime = "nodejs";

function getServiceTokenFromRequest(request: Request): string | null {
  const header = request.headers.get("x-atlas-service-token")?.trim();
  if (header) {
    return header;
  }
  return null;
}

export async function POST(request: Request) {
  const token = getServiceTokenFromRequest(request);
  if (!token) {
    return NextResponse.json({ error: "Missing service token." }, { status: 401 });
  }

  const record = await resolveRunnerServiceToken(token);
  if (!record) {
    return NextResponse.json({ error: "Invalid service token." }, { status: 401 });
  }

  await db
    .update(runnerServiceTokens)
    .set({ lastUsedAt: new Date() })
    .where(eq(runnerServiceTokens.id, record.id));

  const { token: accessToken, expiresIn } = await createRunnerAccessToken({
    packId: record.packId,
    tokenId: record.id,
  });

  return NextResponse.json({
    access_token: accessToken,
    expires_in: expiresIn,
    pack_id: record.packId,
  });
}
