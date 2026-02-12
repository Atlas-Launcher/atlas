import { NextResponse } from "next/server";
import { and, desc, eq, ilike } from "drizzle-orm";

import { auth } from "@/auth";
import { hasRole } from "@/lib/auth/roles";
import { db } from "@/lib/db";
import { accounts, packs } from "@/lib/db/schema";
import { createPackWithDefaults } from "@/lib/packs/create-pack";
import {
  configureRepoForAtlas,
  githubRequest,
  type GithubContentFile,
} from "@/lib/github/repo-config";
import {
  getInstallationTokenForUser,
  GitHubAppNotInstalledError,
  GitHubAppNotConfiguredError,
} from "@/lib/github/app";

type GithubOwner = {
  login: string;
};

type GithubRepo = {
  name: string;
  full_name: string;
  html_url: string;
  clone_url: string;
  owner?: {
    login?: string;
  };
};

function getTemplateRepo() {
  const value = process.env.ATLAS_GITHUB_TEMPLATE_REPO?.trim();
  if (!value) {
    throw new Error("ATLAS_GITHUB_TEMPLATE_REPO is not configured.");
  }

  const [owner, repo] = value.split("/", 2);
  if (!owner || !repo) {
    throw new Error("ATLAS_GITHUB_TEMPLATE_REPO must be in the format 'owner/repo'.");
  }

  return { owner, repo };
}

function getAtlasHubUrl(request: Request) {
  return (
    process.env.ATLAS_HUB_URL?.trim() ??
    process.env.BETTER_AUTH_URL?.trim() ??
    new URL(request.url).origin
  );
}

function normalizeGithubRepoKey(repoUrl: string | null | undefined) {
  if (!repoUrl) {
    return null;
  }

  const trimmed = repoUrl.trim().replace(/\.git$/i, "");
  const httpsMatch = trimmed.match(/github\.com\/([^/]+)\/([^/]+)/i);
  if (httpsMatch) {
    return `${httpsMatch[1].toLowerCase()}/${httpsMatch[2].toLowerCase()}`;
  }
  const sshMatch = trimmed.match(/github\.com:([^/]+)\/([^/]+)/i);
  if (sshMatch) {
    return `${sshMatch[1].toLowerCase()}/${sshMatch[2].toLowerCase()}`;
  }
  return null;
}

async function findExistingOwnedPackByRepoKey(ownerId: string, repoKey: string) {
  const [owner, repo] = repoKey.split("/", 2);
  if (!owner || !repo) {
    return null;
  }

  const candidates = await db
    .select({
      id: packs.id,
      name: packs.name,
      slug: packs.slug,
      description: packs.description,
      repoUrl: packs.repoUrl,
      ownerId: packs.ownerId,
      createdAt: packs.createdAt,
      updatedAt: packs.updatedAt,
    })
    .from(packs)
    .where(and(eq(packs.ownerId, ownerId), ilike(packs.repoUrl, `%github.com/${owner}/${repo}%`)))
    .limit(10);

  return (
    candidates.find((candidate) => normalizeGithubRepoKey(candidate.repoUrl) === repoKey) ??
    null
  );
}

async function createRepositoryFromTemplate({
  token,
  owner,
  name,
  description,
  visibility,
}: {
  token: string;
  owner: string;
  name: string;
  description?: string;
  visibility: "public" | "private";
}) {
  const template = getTemplateRepo();

  return githubRequest<GithubRepo>(
    token,
    `https://api.github.com/repos/${template.owner}/${template.repo}/generate`,
    {
      method: "POST",
      body: JSON.stringify({
        owner,
        name,
        description: description || undefined,
        private: visibility !== "public",
        include_all_branches: false,
      }),
    }
  );
}

async function deleteRepository({
  token,
  owner,
  repo,
}: {
  token: string;
  owner: string;
  repo: string;
}) {
  await githubRequest<null>(token, `https://api.github.com/repos/${owner}/${repo}`, {
    method: "DELETE",
  });
}

export async function GET(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  // Get the user's GitHub username from their linked account
  const [account] = await db
    .select()
    .from(accounts)
    .where(and(eq(accounts.userId, session.user.id), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  if (!account?.accountId) {
    return NextResponse.json(
      {
        error: "No GitHub account linked.",
        code: "GITHUB_NOT_LINKED",
      },
      { status: 404 }
    );
  }

  const { searchParams } = new URL(request.url);
  const page = parseInt(searchParams.get("page") ?? "1", 10);
  const perPage = parseInt(searchParams.get("per_page") ?? "10", 10);
  const search = searchParams.get("search")?.trim();

  // Import the GitHub App utilities
  const { listInstallationRepos } = await import("@/lib/github/app");

  try {
    // Get the user's GitHub username first
    // We need to fetch it since we only store the accountId (numeric ID)
    const userToken = account.accessToken;
    let githubUsername: string | null = null;

    if (userToken) {
      try {
        const userInfo = await githubRequest<GithubOwner>(
          userToken,
          "https://api.github.com/user"
        );
        githubUsername = userInfo.login;
      } catch {
        // Token might be expired, we'll try to look up by ID below
      }
    }

    // If we couldn't get the username from the token, we need to look it up
    // Using the GitHub API's user lookup by ID
    if (!githubUsername) {
      const GITHUB_APP_ID = process.env.GITHUB_APP_ID;
      const GITHUB_APP_PRIVATE_KEY = process.env.GITHUB_APP_PRIVATE_KEY;

      if (!GITHUB_APP_ID || !GITHUB_APP_PRIVATE_KEY) {
        throw new GitHubAppNotConfiguredError();
      }

      // We'll need to get username from another source or throw an error
      return NextResponse.json(
        {
          error: "Unable to determine your GitHub username. Please re-link your GitHub account.",
          code: "GITHUB_USERNAME_UNKNOWN",
        },
        { status: 400 }
      );
    }

    // Get an installation access token for this user
    const installationToken = await getInstallationTokenForUser(githubUsername);

    // List repositories from the installation
    const result = await listInstallationRepos(installationToken, page, perPage);

    // If search is provided, filter the results client-side
    // (GitHub's installation repos endpoint doesn't support search)
    let repos = result.repos;
    if (search) {
      const searchLower = search.toLowerCase();
      repos = repos.filter(
        (repo) =>
          repo.name.toLowerCase().includes(searchLower) ||
          repo.full_name.toLowerCase().includes(searchLower)
      );
    }

    return NextResponse.json({
      repos,
      nextPage: result.hasNextPage ? page + 1 : null,
      totalCount: result.totalCount,
    });
  } catch (error) {
    // Handle specific error types with helpful guidance
    if (error instanceof GitHubAppNotInstalledError) {
      const appSlug = process.env.GITHUB_APP_SLUG || process.env.NEXT_PUBLIC_GITHUB_APP_SLUG || "atlas-launcher";
      return NextResponse.json(
        {
          error: "The Atlas Launcher GitHub App is not installed on your account. Install it to access your repositories.",
          code: "GITHUB_APP_NOT_INSTALLED",
          installUrl: `https://github.com/apps/${appSlug}/installations/new`,
        },
        { status: 403 }
      );
    }

    if (error instanceof GitHubAppNotConfiguredError) {
      return NextResponse.json(
        {
          error: "GitHub App is not configured on this server. Please contact the administrator.",
          code: "GITHUB_APP_NOT_CONFIGURED",
        },
        { status: 500 }
      );
    }

    const errorMessage = error instanceof Error ? error.message : "Unable to load repositories.";
    return NextResponse.json(
      {
        error: errorMessage,
        code: "GITHUB_ERROR",
      },
      { status: 502 }
    );
  }
}

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  // Get the user's GitHub Linked Account
  const [account] = await db
    .select()
    .from(accounts)
    .where(and(eq(accounts.userId, session.user.id), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  if (!account?.accessToken) {
    return NextResponse.json({ error: "No GitHub account linked." }, { status: 404 });
  }

  try {
    getTemplateRepo();
  } catch (error) {
    return NextResponse.json(
      {
        error:
          error instanceof Error
            ? error.message
            : "ATLAS_GITHUB_TEMPLATE_REPO is not configured.",
      },
      { status: 500 }
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

  const repoKey = `${owner.toLowerCase()}/${name.toLowerCase()}`;
  const existingPack = await findExistingOwnedPackByRepoKey(session.user.id, repoKey);
  if (existingPack) {
    return NextResponse.json(
      {
        pack: existingPack,
        alreadyExists: true,
      },
      { status: 200 }
    );
  }

  // Verify access to the requested owner using user token
  try {
    const user = await githubRequest<GithubOwner>(account.accessToken, "https://api.github.com/user");
    const githubUsername = user.login;

    if (owner.toLowerCase() !== githubUsername.toLowerCase()) {
      const orgs = await githubRequest<GithubOwner[]>(account.accessToken, "https://api.github.com/user/orgs?per_page=100");
      const isMember = orgs.some((org) => org.login.toLowerCase() === owner.toLowerCase());
      if (!isMember) {
        return NextResponse.json(
          { error: "Selected owner is not available to this account." },
          { status: 400 }
        );
      }
    }
  } catch {
    return NextResponse.json(
      { error: "Unable to verify GitHub account. Please re-link your GitHub account." },
      { status: 400 }
    );
  }

  // Get installation token for the OWNER
  // For now we use getInstallationTokenForUser(owner). 
  // If owner is an org, this might need adjustment if getInstallationTokenForUser relies on user-specific endpoints.
  // But since we successfully use it for listing repos (which lists user and org repos), it might be robust enough
  // IF the user has installed the app on the org.

  let installationToken: string;
  try {
    installationToken = await getInstallationTokenForUser(owner);
  } catch (error) {
    if (error instanceof GitHubAppNotInstalledError) {
      const appSlug = process.env.GITHUB_APP_SLUG || process.env.NEXT_PUBLIC_GITHUB_APP_SLUG || "atlas-launcher";
      return NextResponse.json(
        {
          error: `The Atlas Launcher GitHub App is not installed on '${owner}'. Install it to allow repository creation.`,
          code: "GITHUB_APP_NOT_INSTALLED",
          installUrl: `https://github.com/apps/${appSlug}/installations/new`,
        },
        { status: 403 }
      );
    }
    throw error;
  }

  let createdPackId: string | null = null;
  let createdRepo: GithubRepo | null = null;

  try {
    // 1. Create the Pack in DB
    const packName = name.replace(/[-_]+/g, " ").trim() || name;
    const createdPack = await createPackWithDefaults({
      ownerId: session.user.id,
      name: packName,
    });
    createdPackId = createdPack.id;

    // 2. Create Repository from Template using INSTALLATION TOKEN
    const repo = await createRepositoryFromTemplate({
      token: installationToken,
      owner,
      name,
      description,
      visibility,
    });
    createdRepo = repo;

    const resolvedOwner = repo?.owner?.login ?? owner;
    const hubUrl = getAtlasHubUrl(request);

    // 3. Configure the Repo (workflows, atlas.toml) using SHARED LOGIC
    // First, check if atlas.toml exists (it should, from template)
    const { checkAtlasTomlExists, ensureAtlasBuildWorkflow, setRepositoryActionsPermissions, enableRepositoryWorkflows } = await import("@/lib/github/repo-config");

    // We add a small retry loop for searching the file, just in case of propagation delay
    let atlasToml: GithubContentFile | null = null;
    for (let i = 0; i < 3; i++) {
      atlasToml = await checkAtlasTomlExists({
        token: installationToken,
        owner: resolvedOwner,
        repo: repo.name,
      });
      if (atlasToml) break;
      await new Promise(r => setTimeout(r, 500));
    }

    if (atlasToml) {
      await configureRepoForAtlas({
        token: installationToken,
        owner: resolvedOwner,
        repo: repo.name,
        packId: createdPack.id,
        hubUrl,
        existingAtlasToml: atlasToml,
      });
    } else {
      // Fallback if atlas.toml missing in template: just ensure workflow and permissions
      await ensureAtlasBuildWorkflow({ token: installationToken, owner: resolvedOwner, repo: repo.name });
      await setRepositoryActionsPermissions({ token: installationToken, owner: resolvedOwner, repo: repo.name });
      await enableRepositoryWorkflows({ token: installationToken, owner: resolvedOwner, repo: repo.name });
    }

    // 4. Update Pack with Repo URL
    const [updatedPack] = await db
      .update(packs)
      .set({
        repoUrl: repo.html_url,
        updatedAt: new Date(),
      })
      .where(eq(packs.id, createdPack.id))
      .returning();

    return NextResponse.json(
      {
        repo: {
          name: repo.name,
          fullName: repo.full_name,
          htmlUrl: repo.html_url,
          cloneUrl: repo.clone_url,
          owner: repo?.owner?.login ?? owner,
        },
        pack: updatedPack ?? createdPack,
      },
      { status: 201 }
    );
  } catch (error) {
    // Cleanup on failure
    if (createdRepo) {
      const repoOwner = createdRepo?.owner?.login ?? owner;
      if (repoOwner) {
        // Delete using installation token
        await deleteRepository({
          token: installationToken,
          owner: repoOwner,
          repo: createdRepo.name,
        }).catch(() => null);
      }
    }

    if (createdPackId) {
      await db.delete(packs).where(eq(packs.id, createdPackId)).catch(() => null);
    }

    return NextResponse.json(
      {
        error:
          error instanceof Error
            ? `${error.message}. Ensure the GitHub App is installed and has access to the target owner.`
            : "Unable to create GitHub repository. Check your GitHub App installation and permissions.",
      },
      { status: 502 }
    );
  }
}
