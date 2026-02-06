import { and, eq, gt } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { oauthAccessTokens, sessions } from "@/lib/db/schema";

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

  return getAuthenticatedUserIdFromBearerToken(bearer);
}

export async function getAuthenticatedUserIdFromBearerToken(
  bearerToken: string
): Promise<string | null> {
  const tokenValue = bearerToken.trim();
  if (!tokenValue) {
    return null;
  }

  type AuthApiLike = {
    getMcpSession?: (input: { headers: Headers }) => Promise<{ userId?: string | null } | null>;
  };
  const mcpSessionGetter = (auth.api as AuthApiLike).getMcpSession;
  if (typeof mcpSessionGetter === "function") {
    const mcpSession = await mcpSessionGetter({
      headers: new Headers({
        authorization: `Bearer ${tokenValue}`,
      }),
    }).catch(() => null);
    if (mcpSession?.userId) {
      return mcpSession.userId;
    }
  }

  // Some flows store auth in session cookies rather than oauthAccessToken rows.
  const sessionCookie = "better-auth.session_token";
  const sessionFromBearer = await auth.api
    .getSession({
      headers: new Headers({
        cookie: `${sessionCookie}=${tokenValue}`,
      }),
    })
    .catch(() => null);
  if (sessionFromBearer?.user?.id) {
    return sessionFromBearer.user.id;
  }

  const [token] = await db
    .select({
      userId: oauthAccessTokens.userId,
    })
    .from(oauthAccessTokens)
    .where(
      and(
        eq(oauthAccessTokens.accessToken, tokenValue),
        gt(oauthAccessTokens.accessTokenExpiresAt, new Date())
      )
    )
    .limit(1);

  if (token?.userId) {
    return token.userId;
  }

  const [sessionToken] = await db
    .select({
      userId: sessions.userId,
    })
    .from(sessions)
    .where(and(eq(sessions.token, tokenValue), gt(sessions.expiresAt, new Date())))
    .limit(1);

  return sessionToken?.userId ?? null;
}
