import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { accounts } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";

export async function GET(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [account] = await db
    .select({
      accessToken: accounts.accessToken,
    })
    .from(accounts)
    .where(
      eq(accounts.userId, userId) &&
      eq(accounts.providerId, "github")
    )
    .orderBy(accounts.updatedAt)
    .limit(1);

  if (!account?.accessToken) {
    return NextResponse.json({ error: "No linked GitHub account found" }, { status: 404 });
  }

  return NextResponse.json({
    access_token: account.accessToken,
  });
}