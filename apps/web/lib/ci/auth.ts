import { and, eq, gt } from "drizzle-orm";

import { db } from "@/lib/db";
import { oauthAccessTokens, packMembers, packs, users } from "@/lib/db/schema";
import { toRepositorySlug } from "@/lib/github";
import { verifyGithubOidcToken, type GithubOidcClaims } from "@/lib/ci/oidc";

type CiAuthContext = {
  packId: string;
  method: "github_oidc" | "launcher_user";
  repository?: string;
  oidcClaims?: GithubOidcClaims;
  userId?: string;
};

function getHeaderToken(request: Request, name: string): string | null {
  const value = request.headers.get(name)?.trim();
  return value || null;
}

function parseBearerToken(request: Request): string | null {
  const header = getHeaderToken(request, "authorization");
  if (!header) {
    return null;
  }

  const [scheme, token] = header.split(/\s+/, 2);
  if (!scheme || !token || scheme.toLowerCase() !== "bearer") {
    return null;
  }

  return token.trim() || null;
}

export async function resolveCiAuthContext(
  request: Request,
  requestedPackId: string | null
): Promise<CiAuthContext> {
  const oidcToken = getHeaderToken(request, "x-atlas-oidc-token");
  if (oidcToken) {
    return authorizeGithubOidcToken(requestedPackId, oidcToken);
  }

  const bearer = parseBearerToken(request);
  if (bearer) {
    return authorizeLauncherUserToken(requestedPackId, bearer);
  }

  throw new Error("Missing CI credentials.");
}

async function authorizeLauncherUserToken(
  requestedPackId: string | null,
  bearerToken: string
): Promise<CiAuthContext> {
  const packId = requestedPackId?.toString().trim();
  if (!packId) {
    throw new Error("packId is required when using user credentials.");
  }

  const [token] = await db
    .select({
      userId: oauthAccessTokens.userId,
    })
    .from(oauthAccessTokens)
    .where(
      and(
        eq(oauthAccessTokens.accessToken, bearerToken),
        gt(oauthAccessTokens.accessTokenExpiresAt, new Date())
      )
    )
    .limit(1);

  if (!token?.userId) {
    throw new Error("Invalid bearer token.");
  }

  const [user] = await db
    .select({
      role: users.role,
    })
    .from(users)
    .where(eq(users.id, token.userId))
    .limit(1);

  if (user?.role === "admin") {
    return {
      packId,
      method: "launcher_user",
      userId: token.userId,
    };
  }

  const [membership] = await db
    .select({
      role: packMembers.role,
    })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, token.userId)))
    .limit(1);

  if (membership?.role !== "admin" && membership?.role !== "creator") {
    throw new Error("User does not have deploy permission for this pack.");
  }

  return {
    packId,
    method: "launcher_user",
    userId: token.userId,
  };
}

async function authorizeGithubOidcToken(
  requestedPackId: string | null,
  token: string
): Promise<CiAuthContext> {
  const packId = requestedPackId?.trim();
  if (!packId) {
    throw new Error("packId is required when authenticating with OIDC.");
  }

  const expectedAudience = process.env.ATLAS_CI_OIDC_AUDIENCE?.trim() || "atlas-hub";
  const claims = await verifyGithubOidcToken(token, expectedAudience);

  const [pack] = await db
    .select({
      id: packs.id,
      repoUrl: packs.repoUrl,
    })
    .from(packs)
    .where(eq(packs.id, packId))
    .limit(1);

  if (!pack) {
    throw new Error("Unknown pack.");
  }

  const expectedRepository = toRepositorySlug(pack.repoUrl);
  if (!expectedRepository) {
    throw new Error("Pack does not have a valid GitHub repository URL.");
  }

  const tokenRepository = claims.repository?.toString().toLowerCase();
  if (!tokenRepository || tokenRepository !== expectedRepository) {
    throw new Error("OIDC repository claim does not match pack repository.");
  }

  const subject = claims.sub?.toString();
  const expectedSubjectPrefix = `repo:${expectedRepository}:`;
  if (!subject?.toLowerCase().startsWith(expectedSubjectPrefix)) {
    throw new Error("OIDC subject claim does not match expected repository subject.");
  }

  const expectedWorkflowPath = process.env.ATLAS_CI_WORKFLOW_PATH?.trim();
  const workflowRef = claims.job_workflow_ref?.toString();
  if (expectedWorkflowPath && workflowRef) {
    const normalizedRepo = expectedRepository.toLowerCase();
    const normalizedPath = expectedWorkflowPath.replace(/^\/+/, "");
    const expectedWorkflowPrefix = `${normalizedRepo}/${normalizedPath}@`;
    if (!workflowRef.toLowerCase().startsWith(expectedWorkflowPrefix)) {
      throw new Error("OIDC token was not issued from the configured CI workflow path.");
    }
  }

  return {
    packId,
    method: "github_oidc",
    repository: expectedRepository,
    oidcClaims: claims,
  };
}
