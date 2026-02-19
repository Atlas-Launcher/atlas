import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { accounts, packMembers, packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
import { toRepositorySlug } from "@/lib/github";
import { emitWhitelistUpdate } from "@/lib/whitelist-events";
import { recomputeWhitelist } from "@/lib/packs/whitelist";

interface RouteParams {
  params: Promise<{
    packId: string;
    userId: string;
  }>;
}

type GithubUser = {
  login?: string;
};

type GithubRepositoryInvitation = {
  id: number;
  repository?: {
    full_name?: string;
  };
};

async function githubRequest(
  token: string,
  url: string,
  init?: RequestInit
): Promise<Response> {
  return fetch(url, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    cache: "no-store",
  });
}

async function parseGithubError(response: Response) {
  const body = await response.json().catch(() => ({}));
  const message =
    typeof body?.message === "string" && body.message.trim()
      ? body.message.trim()
      : `GitHub request failed with status ${response.status}.`;
  return message;
}

async function getPackManagerGithubToken(userId: string): Promise<string | null> {
  const [account] = await db
    .select({
      accessToken: accounts.accessToken,
    })
    .from(accounts)
    .where(and(eq(accounts.userId, userId), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  return account?.accessToken?.trim() || null;
}

async function getTargetGithubAccount(userId: string) {
  const [account] = await db
    .select({
      accountId: accounts.accountId,
      accessToken: accounts.accessToken,
      accessTokenExpiresAt: accounts.accessTokenExpiresAt,
    })
    .from(accounts)
    .where(and(eq(accounts.userId, userId), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  return account ?? null;
}

async function resolveGithubLoginByAccountId(
  managerToken: string,
  accountId: string
): Promise<string> {
  const response = await githubRequest(
    managerToken,
    `https://api.github.com/user/${encodeURIComponent(accountId)}`
  );
  if (!response.ok) {
    throw new Error(await parseGithubError(response));
  }

  const payload = (await response.json()) as GithubUser;
  const login = payload.login?.trim();
  if (!login) {
    throw new Error("Unable to determine GitHub username for this member.");
  }
  return login;
}

async function resolveGithubLoginForMember({
  managerToken,
  accountId,
  targetToken,
  targetTokenExpiresAt,
}: {
  managerToken: string;
  accountId?: string | null;
  targetToken?: string | null;
  targetTokenExpiresAt?: Date | null;
}): Promise<string> {
  if (targetToken && (!targetTokenExpiresAt || targetTokenExpiresAt > new Date())) {
    try {
      return await resolveGithubLogin(targetToken);
    } catch {
      // Fall through to account-id lookup.
    }
  }

  const normalizedAccountId = accountId?.trim();
  if (!normalizedAccountId) {
    throw new Error("User does not have enough GitHub identity data for repository cleanup.");
  }

  return resolveGithubLoginByAccountId(managerToken, normalizedAccountId);
}

async function resolveGithubLogin(targetToken: string): Promise<string> {
  const response = await githubRequest(targetToken, "https://api.github.com/user");
  if (!response.ok) {
    throw new Error(await parseGithubError(response));
  }

  const payload = (await response.json()) as GithubUser;
  const login = payload.login?.trim();
  if (!login) {
    throw new Error("Unable to determine GitHub username for the promoted user.");
  }
  return login;
}

async function acceptRepositoryInvitation(
  targetToken: string,
  invitationId: number
): Promise<boolean> {
  const response = await githubRequest(
    targetToken,
    `https://api.github.com/user/repository_invitations/${invitationId}`,
    {
      method: "PATCH",
    }
  );
  return response.ok;
}

async function bestEffortAcceptByRepoSlug(
  targetToken: string,
  repoSlug: string
): Promise<boolean> {
  const response = await githubRequest(
    targetToken,
    "https://api.github.com/user/repository_invitations?per_page=100"
  );
  if (!response.ok) {
    return false;
  }

  const invitations = (await response.json()) as GithubRepositoryInvitation[];
  const target = invitations.find(
    (invitation) =>
      invitation.repository?.full_name?.toLowerCase() === repoSlug.toLowerCase()
  );
  if (!target?.id) {
    return false;
  }

  return acceptRepositoryInvitation(targetToken, target.id);
}

async function addCreatorToRepository({
  repoSlug,
  managerToken,
  targetToken,
  accountId,
}: {
  repoSlug: string;
  managerToken: string;
  targetToken?: string | null;
  accountId?: string | null;
}): Promise<{ inviteAccepted: boolean }> {
  const login = await resolveGithubLoginForMember({
    managerToken,
    accountId,
    targetToken,
  });
  const inviteResponse = await githubRequest(
    managerToken,
    `https://api.github.com/repos/${repoSlug}/collaborators/${encodeURIComponent(login)}`,
    {
      method: "PUT",
      body: JSON.stringify({
        permission: "push",
      }),
    }
  );

  if (!inviteResponse.ok) {
    throw new Error(await parseGithubError(inviteResponse));
  }

  if (inviteResponse.status === 204) {
    return { inviteAccepted: true };
  }

  let inviteAccepted = false;
  if (inviteResponse.status === 201 && targetToken) {
    const payload = (await inviteResponse.json().catch(() => null)) as
      | GithubRepositoryInvitation
      | null;
    if (payload?.id) {
      inviteAccepted = await acceptRepositoryInvitation(targetToken, payload.id);
    }
    if (!inviteAccepted) {
      inviteAccepted = await bestEffortAcceptByRepoSlug(targetToken, repoSlug);
    }
  }

  return { inviteAccepted };
}

async function removeCreatorFromRepository({
  repoSlug,
  managerToken,
  login,
}: {
  repoSlug: string;
  managerToken: string;
  login: string;
}): Promise<void> {
  const response = await githubRequest(
    managerToken,
    `https://api.github.com/repos/${repoSlug}/collaborators/${encodeURIComponent(login)}`,
    {
      method: "DELETE",
    }
  );
  if (response.status === 204 || response.status === 404) {
    return;
  }
  if (!response.ok) {
    throw new Error(await parseGithubError(response));
  }
}

async function canManagePackMembers(packId: string, userId: string, isAdmin: boolean) {
  if (isAdmin) {
    return true;
  }

  const [membership] = await db
    .select({ role: packMembers.role })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)));
  return Boolean(membership && membership.role !== "player");
}

export async function PATCH(request: Request, { params }: RouteParams) {
  const { packId, userId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const isAdmin = hasRole(session, ["admin"]);
  const canManage = await canManagePackMembers(packId, session.user.id, isAdmin);
  if (!canManage) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json().catch(() => ({}));
  const roleInput = body?.role?.toString().trim().toLowerCase();
  const role =
    roleInput === "player" || roleInput === "creator" ? roleInput : undefined;
  const accessInput = body?.accessLevel?.toString().trim().toLowerCase();
  const accessLevel =
    accessInput === "dev" ||
      accessInput === "beta" ||
      accessInput === "production" ||
      accessInput === "all"
      ? accessInput
      : undefined;
  if (!role && !accessLevel) {
    return NextResponse.json(
      { error: "Provide role and/or accessLevel to update a member." },
      { status: 400 }
    );
  }

  const [existing] = await db
    .select({
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
    })
    .from(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .limit(1);

  if (!existing) {
    return NextResponse.json({ error: "Member not found." }, { status: 404 });
  }

  const warnings: string[] = [];
  let inviteAccepted = false;
  let collaboratorRemoved = false;

  if (role === "creator" && existing.role !== "creator") {
    const [pack] = await db
      .select({
        repoUrl: packs.repoUrl,
      })
      .from(packs)
      .where(eq(packs.id, packId))
      .limit(1);

    const repoSlug = toRepositorySlug(pack?.repoUrl);
    if (!repoSlug) {
      return NextResponse.json(
        {
          error:
            "Pack repository is not linked to GitHub. Link a GitHub repository before promoting creators.",
        },
        { status: 400 }
      );
    }

    const targetGithub = await getTargetGithubAccount(userId);
    const targetAccountId = targetGithub?.accountId?.trim() || null;
    if (!targetAccountId) {
      warnings.push(
        "This user does not have a linked GitHub account. You must add them to the GitHub repository manually."
      );
    } else {
      const targetToken = targetGithub?.accessToken?.trim() || null;
      const isTokenExpired =
        targetGithub?.accessTokenExpiresAt &&
        targetGithub.accessTokenExpiresAt <= new Date();

      const managerToken = await getPackManagerGithubToken(session.user.id);
      if (!managerToken) {
        return NextResponse.json(
          {
            error:
              "Your account must be linked to GitHub to promote creators for this pack repository.",
          },
          { status: 400 }
        );
      }

      try {
        const syncResult = await addCreatorToRepository({
          repoSlug,
          managerToken,
          targetToken: isTokenExpired ? null : targetToken,
          accountId: targetAccountId,
        });
        inviteAccepted = syncResult.inviteAccepted;
      } catch (error) {
        return NextResponse.json(
          {
            error:
              error instanceof Error
                ? `Failed to add user to GitHub repository: ${error.message}`
                : "Failed to add user to GitHub repository.",
          },
          { status: 502 }
        );
      }
    }
  }

  if (role === "player" && existing.role === "creator") {
    const [pack] = await db
      .select({
        repoUrl: packs.repoUrl,
      })
      .from(packs)
      .where(eq(packs.id, packId))
      .limit(1);

    const repoSlug = toRepositorySlug(pack?.repoUrl);
    if (repoSlug) {
      const managerToken = await getPackManagerGithubToken(session.user.id);
      if (!managerToken) {
        return NextResponse.json(
          {
            error:
              "Your account must be linked to GitHub to demote creators and remove repository access.",
          },
          { status: 400 }
        );
      }

      const targetGithub = await getTargetGithubAccount(userId);
      if (!targetGithub) {
        return NextResponse.json(
          {
            error:
              "User has no linked GitHub account record, so repository access cannot be removed automatically.",
          },
          { status: 400 }
        );
      }

      try {
        const login = await resolveGithubLoginForMember({
          managerToken,
          accountId: targetGithub.accountId,
          targetToken: targetGithub.accessToken,
          targetTokenExpiresAt: targetGithub.accessTokenExpiresAt,
        });
        await removeCreatorFromRepository({
          repoSlug,
          managerToken,
          login,
        });
        collaboratorRemoved = true;
      } catch (error) {
        return NextResponse.json(
          {
            error:
              error instanceof Error
                ? `Failed to remove user from GitHub repository: ${error.message}`
                : "Failed to remove user from GitHub repository.",
          },
          { status: 502 }
        );
      }
    }
  }

  const nextRole = role ?? existing.role;
  const nextAccessLevel =
    accessLevel ?? (role ? (role === "creator" ? "all" : "production") : existing.accessLevel);

  const [updated] = await db
    .update(packMembers)
    .set({
      role: nextRole,
      accessLevel: nextAccessLevel,
    })
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)))
    .returning({
      userId: packMembers.userId,
      role: packMembers.role,
      accessLevel: packMembers.accessLevel,
    });

  if (!updated) {
    return NextResponse.json({ error: "Member not found." }, { status: 404 });
  }

  return NextResponse.json({
    ok: true,
    member: updated,
    warnings,
    github: {
      inviteAccepted,
      collaboratorRemoved,
    },
  });
}

export async function DELETE(request: Request, { params }: RouteParams) {
  const { packId, userId } = await params;
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const isAdmin = hasRole(session, ["admin"]);
  const canManage = await canManagePackMembers(packId, session.user.id, isAdmin);
  if (!canManage) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  await db
    .delete(packMembers)
    .where(and(eq(packMembers.packId, packId), eq(packMembers.userId, userId)));

  await recomputeWhitelist(packId);

  emitWhitelistUpdate({ packId, source: "member-remove" });

  return NextResponse.json({ ok: true });
}
