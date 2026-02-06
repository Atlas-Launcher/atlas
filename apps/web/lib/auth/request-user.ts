import { and, eq, gt } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { oauthAccessTokens } from "@/lib/db/schema";

function parseBearerToken(headers: Headers): string | null {
  const header = headers.get("authorization")?.trim();
  if (!header) {
    return null;
  }
  const [scheme, token] = header.split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }
  return token.trim() || null;
}

export async function getAuthenticatedUserId(request: Request): Promise<string | null> {
  const session = await auth.api.getSession({ headers: request.headers });
  if (session?.user?.id) {
    return session.user.id;
  }

  const bearer = parseBearerToken(request.headers);
  if (!bearer) {
    return null;
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

  return token?.userId ?? null;
}
