import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { accounts } from "@/lib/db/schema";
import { getAtlasPackTemplateFiles } from "@/lib/templates/atlas-pack";

function slugify(value: string) {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-");
}

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

async function githubRequest<T>(
  token: string,
  url: string,
  init?: RequestInit
): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      Accept: "application/vnd.github+json",
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
    cache: "no-store",
  });

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body?.message ?? "GitHub request failed");
  }

  return response.json();
}

function encodePath(pathValue: string) {
  return pathValue
    .split("/")
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

function applyTemplateVariables(content: string, values: Record<string, string>) {
  return Object.entries(values).reduce((acc, [key, value]) => {
    return acc.replaceAll(`{{${key}}}`, value);
  }, content);
}

async function seedRepositoryWithTemplate({
  token,
  owner,
  repo,
  repoName,
}: {
  token: string;
  owner: string;
  repo: string;
  repoName: string;
}) {
  const files = await getAtlasPackTemplateFiles();
  const packName = repoName.replace(/[-_]+/g, " ").trim() || "Atlas Pack";
  const replacements = {
    PACK_NAME: packName,
    PACK_SLUG: slugify(repoName),
    REPO_NAME: repoName,
    OWNER: owner,
    PACK_ID: "",
  };
  const commitMessage = "Initialize Atlas pack template";

  for (const file of files) {
    const content = applyTemplateVariables(file.content, replacements);
    const encodedPath = encodePath(file.path);
    await githubRequest(token, `https://api.github.com/repos/${owner}/${repo}/contents/${encodedPath}`, {
      method: "PUT",
      body: JSON.stringify({
        message: commitMessage,
        content: Buffer.from(content).toString("base64"),
      }),
    });
  }
}

export async function POST(request: Request) {
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

  const body = await request.json();
  const owner = body?.owner?.toString().trim();
  const name = body?.name?.toString().trim();
  const description = body?.description?.toString().trim();
  const visibility = body?.visibility === "public" ? "public" : "private";

  if (!owner || !name) {
    return NextResponse.json(
      { error: "Owner and repository name are required." },
      { status: 400 }
    );
  }

  try {
    const [user, orgs] = await Promise.all([
      githubRequest<{ login: string }>(token, "https://api.github.com/user"),
      githubRequest<Array<{ login: string }>>(
        token,
        "https://api.github.com/user/orgs?per_page=100"
      ),
    ]);

    const ownerIsUser = owner.toLowerCase() === user.login.toLowerCase();
    const ownerIsOrg = orgs.some(
      (org) => org.login.toLowerCase() === owner.toLowerCase()
    );

    if (!ownerIsUser && !ownerIsOrg) {
      return NextResponse.json(
        { error: "Selected owner is not available to this account." },
        { status: 400 }
      );
    }

    const payload = {
      name,
      description: description || undefined,
      private: visibility !== "public",
    };

    const url = ownerIsUser
      ? "https://api.github.com/user/repos"
      : `https://api.github.com/orgs/${owner}/repos`;

    const repo = await githubRequest<any>(token, url, {
      method: "POST",
      body: JSON.stringify(payload),
    });

    let warning: string | null = null;
    try {
      await seedRepositoryWithTemplate({
        token,
        owner: repo?.owner?.login ?? owner,
        repo: repo.name,
        repoName: repo.name,
      });
    } catch (error) {
      warning =
        error instanceof Error
          ? error.message
          : "Repository created, but template seeding failed.";
    }

    return NextResponse.json(
      {
        repo: {
          name: repo.name,
          fullName: repo.full_name,
          htmlUrl: repo.html_url,
          cloneUrl: repo.clone_url,
          owner: repo?.owner?.login,
        },
        warning,
      },
      { status: 201 }
    );
  } catch (error) {
    return NextResponse.json(
      {
        error:
          error instanceof Error
            ? error.message
            : "Unable to create GitHub repository.",
      },
      { status: 502 }
    );
  }
}
