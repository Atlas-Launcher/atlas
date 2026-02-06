import { NextResponse } from "next/server";
import { and, desc, eq, gt } from "drizzle-orm";

import { db } from "@/lib/db";
import { accounts, oauthAccessTokens } from "@/lib/db/schema";

function parseBearerToken(request: Request): string | null {
  const header = request.headers.get("authorization")?.trim();
  if (!header) {
    return null;
  }
  const [scheme, token] = header.split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }
  return token.trim() || null;
}

export async function GET(request: Request) {
  const bearer = parseBearerToken(request);
  if (!bearer) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [token] = await db
    .select({
      userId: oauthAccessTokens.userId,
    })
    .from(oauthAccessTokens)
    .where(
      and(
        eq(oauthAccessTokens.accessToken, bearer),
        gt(oauthAccessTokens.accessTokenExpiresAt, new Date())
      )
    )
    .limit(1);

  if (!token?.userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [githubAccount] = await db
    .select({
      accessToken: accounts.accessToken,
      accessTokenExpiresAt: accounts.accessTokenExpiresAt,
    })
    .from(accounts)
    .where(and(eq(accounts.userId, token.userId), eq(accounts.providerId, "github")))
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
