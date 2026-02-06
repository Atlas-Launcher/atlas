import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import { accounts } from "@/lib/db/schema";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";

export async function GET(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [githubAccount] = await db
    .select({
      accessToken: accounts.accessToken,
      accessTokenExpiresAt: accounts.accessTokenExpiresAt,
    })
    .from(accounts)
    .where(and(eq(accounts.userId, userId), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  if (!githubAccount?.accessToken) {
    return NextResponse.json({ error: "No linked GitHub account." }, { status: 404 });
  }

  if (
    githubAccount.accessTokenExpiresAt &&
    githubAccount.accessTokenExpiresAt <= new Date()
  ) {
    return NextResponse.json(
      { error: "Linked GitHub token has expired. Re-link your GitHub account." },
      { status: 409 }
    );
  }

  return NextResponse.json({ accessToken: githubAccount.accessToken });
}
