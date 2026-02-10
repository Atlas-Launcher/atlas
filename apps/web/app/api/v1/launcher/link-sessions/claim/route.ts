import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { launcherLinkSessions } from "@/lib/db/schema";

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => ({}));
  const code = body?.code?.toString().trim();

  if (!code) {
    return NextResponse.json({ error: "Code is required" }, { status: 400 });
  }

  const [linkSession] = await db
    .select()
    .from(launcherLinkSessions)
    .where(eq(launcherLinkSessions.code, code));

  if (!linkSession) {
    return NextResponse.json({ error: "Link session not found" }, { status: 404 });
  }

  if (linkSession.expiresAt < new Date()) {
    return NextResponse.json({ error: "Link session expired" }, { status: 410 });
  }

  if (linkSession.completedAt) {
    return NextResponse.json({ error: "Link session already completed" }, { status: 409 });
  }

  if (linkSession.claimedUserId && linkSession.claimedUserId !== session.user.id) {
    return NextResponse.json({ error: "Link session already claimed" }, { status: 409 });
  }

  await db
    .update(launcherLinkSessions)
    .set({
      claimedUserId: session.user.id,
      claimedAt: linkSession.claimedAt ?? new Date(),
    })
    .where(eq(launcherLinkSessions.id, linkSession.id));

  return NextResponse.json({
    success: true,
    linkSessionId: linkSession.id,
  });
}
