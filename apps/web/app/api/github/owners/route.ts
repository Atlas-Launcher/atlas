import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { accounts } from "@/lib/db/schema";

async function getGithubToken(userId: string) {
  const [account] = await db
    .select()
    .from(accounts)
    .where(and(eq(accounts.userId, userId), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  if (!account?.accessToken) {
    return null;
  }

  return account.accessToken;
}

async function githubRequest<T>(token: string, url: string): Promise<T> {
  const response = await fetch(url, {
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
    },
    cache: "no-store",
  });

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body?.message ?? "GitHub request failed");
  }

  return response.json();
}

export async function GET(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const token = await getGithubToken(session.user.id);

  if (!token) {
    return NextResponse.json(
      { error: "No GitHub account linked." },
      { status: 404 }
    );
  }

  try {
    const [user, orgs] = await Promise.all([
      githubRequest<{ login: string; avatar_url?: string }>(
        token,
        "https://api.github.com/user"
      ),
      githubRequest<Array<{ login: string; avatar_url?: string }>>(
        token,
        "https://api.github.com/user/orgs?per_page=100"
      ),
    ]);

    const owners = [
      {
        login: user.login,
        type: "user",
        avatarUrl: user.avatar_url ?? null,
      },
      ...orgs.map((org) => ({
        login: org.login,
        type: "org",
        avatarUrl: org.avatar_url ?? null,
      })),
    ];

    return NextResponse.json({ owners });
  } catch (error) {
    return NextResponse.json(
      {
        error:
          error instanceof Error
            ? `${error.message}. Ensure the GitHub App is installed on your account or organization.`
            : "Unable to load GitHub owners. Check your GitHub App installation.",
      },
      { status: 502 }
    );
  }
}
