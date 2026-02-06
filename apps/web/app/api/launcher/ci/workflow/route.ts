import { NextResponse } from "next/server";
import { and, desc, eq, gt } from "drizzle-orm";

import { db } from "@/lib/db";
import { accounts, oauthAccessTokens, packMembers, packs } from "@/lib/db/schema";
import { toRepositorySlug } from "@/lib/github";

type WorkflowSyncAction = "init" | "update";

type GithubContentFile = {
  sha: string;
  content: string;
  encoding: string;
};

type GithubWorkflow = {
  id: number;
  state: string;
};

type GithubWorkflowListResponse = {
  workflows?: GithubWorkflow[];
};

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

function getWorkflowPath() {
  return process.env.ATLAS_CI_WORKFLOW_PATH?.trim() || ".github/workflows/atlas-build.yml";
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

  if (response.status === 204) {
    return null as T;
  }

  const text = await response.text();
  if (!text) {
    return null as T;
  }

  return JSON.parse(text) as T;
}

async function getRepositoryFile(
  token: string,
  owner: string,
  repo: string,
  path: string
): Promise<GithubContentFile | null> {
  const response = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/contents/${encodeURIComponent(path)}`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/vnd.github+json",
      },
      cache: "no-store",
    }
  );

  if (response.status === 404) {
    return null;
  }

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body?.message ?? "GitHub request failed");
  }

  return (await response.json()) as GithubContentFile;
}

function decodeBase64Content(content: string) {
  return Buffer.from(content.replace(/\n/g, ""), "base64").toString("utf8");
}

async function upsertRepositoryFile({
  token,
  owner,
  repo,
  path,
  content,
  message,
  sha,
}: {
  token: string;
  owner: string;
  repo: string;
  path: string;
  content: string;
  message: string;
  sha?: string;
}) {
  await githubRequest<null>(
    token,
    `https://api.github.com/repos/${owner}/${repo}/contents/${encodeURIComponent(path)}`,
    {
      method: "PUT",
      body: JSON.stringify({
        message,
        content: Buffer.from(content, "utf8").toString("base64"),
        ...(sha ? { sha } : {}),
      }),
    }
  );
}

async function deleteRepositorySecret({
  token,
  owner,
  repo,
  name,
}: {
  token: string;
  owner: string;
  repo: string;
  name: string;
}) {
  await fetch(
    `https://api.github.com/repos/${owner}/${repo}/actions/secrets/${encodeURIComponent(name)}`,
    {
      method: "DELETE",
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "application/vnd.github+json",
      },
      cache: "no-store",
    }
  );
}

async function setRepositoryActionsPermissions({
  token,
  owner,
  repo,
}: {
  token: string;
  owner: string;
  repo: string;
}) {
  await githubRequest<null>(
    token,
    `https://api.github.com/repos/${owner}/${repo}/actions/permissions`,
    {
      method: "PUT",
      body: JSON.stringify({
        enabled: true,
        allowed_actions: "all",
      }),
    }
  );
}

async function listRepositoryWorkflows({
  token,
  owner,
  repo,
}: {
  token: string;
  owner: string;
  repo: string;
}) {
  const response = await githubRequest<GithubWorkflowListResponse>(
    token,
    `https://api.github.com/repos/${owner}/${repo}/actions/workflows?per_page=100`
  );
  return Array.isArray(response.workflows) ? response.workflows : [];
}

async function enableRepositoryWorkflows({
  token,
  owner,
  repo,
}: {
  token: string;
  owner: string;
  repo: string;
}) {
  const workflows = await listRepositoryWorkflows({ token, owner, repo });
  await Promise.all(
    workflows.map(async (workflow) => {
      if (workflow.state === "active") {
        return;
      }

      await githubRequest<null>(
        token,
        `https://api.github.com/repos/${owner}/${repo}/actions/workflows/${encodeURIComponent(
          String(workflow.id)
        )}/enable`,
        {
          method: "PUT",
        }
      );
    })
  );
}

export async function POST(request: Request) {
  const launcherClientId =
    process.env.ATLAS_OIDC_LAUNCHER_CLIENT_ID ?? "atlas-launcher";
  const bearer = parseBearerToken(request);
  if (!bearer) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [oauthToken] = await db
    .select({
      userId: oauthAccessTokens.userId,
    })
    .from(oauthAccessTokens)
    .where(
      and(
        eq(oauthAccessTokens.accessToken, bearer),
        eq(oauthAccessTokens.clientId, launcherClientId),
        gt(oauthAccessTokens.accessTokenExpiresAt, new Date())
      )
    )
    .limit(1);

  if (!oauthToken?.userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const body = await request.json().catch(() => ({}));
  const action = body?.action?.toString() as WorkflowSyncAction | undefined;
  const packId = body?.packId?.toString().trim();

  if (!packId) {
    return NextResponse.json({ error: "packId is required." }, { status: 400 });
  }
  if (action !== "init" && action !== "update") {
    return NextResponse.json({ error: "action must be 'init' or 'update'." }, { status: 400 });
  }

  const [membership] = await db
    .select({
      role: packMembers.role,
      packId: packs.id,
      repoUrl: packs.repoUrl,
    })
    .from(packMembers)
    .innerJoin(packs, eq(packMembers.packId, packs.id))
    .where(and(eq(packMembers.userId, oauthToken.userId), eq(packMembers.packId, packId)))
    .limit(1);

  if (!membership) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }
  if (membership.role !== "admin" && membership.role !== "creator") {
    return NextResponse.json({ error: "Creator or admin role is required." }, { status: 403 });
  }

  const repoSlug = toRepositorySlug(membership.repoUrl);
  if (!repoSlug) {
    return NextResponse.json(
      { error: "Pack is missing a valid GitHub repository URL." },
      { status: 400 }
    );
  }

  const [owner, repo] = repoSlug.split("/", 2);
  if (!owner || !repo) {
    return NextResponse.json({ error: "Invalid repository mapping." }, { status: 400 });
  }

  const [account] = await db
    .select({
      accessToken: accounts.accessToken,
    })
    .from(accounts)
    .where(and(eq(accounts.userId, oauthToken.userId), eq(accounts.providerId, "github")))
    .orderBy(desc(accounts.updatedAt))
    .limit(1);

  const githubToken = account?.accessToken?.trim();
  if (!githubToken) {
    return NextResponse.json({ error: "No linked GitHub account." }, { status: 404 });
  }

  let templateRepo;
  try {
    templateRepo = getTemplateRepo();
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Template repository is not configured." },
      { status: 500 }
    );
  }

  const workflowPath = getWorkflowPath();

  try {
    const templateWorkflow = await getRepositoryFile(
      githubToken,
      templateRepo.owner,
      templateRepo.repo,
      workflowPath
    );
    if (!templateWorkflow?.content || templateWorkflow.encoding !== "base64") {
      throw new Error("Template workflow file was not found or had an unsupported encoding.");
    }
    const templateContent = decodeBase64Content(templateWorkflow.content);

    const existingWorkflow = await getRepositoryFile(githubToken, owner, repo, workflowPath);
    let workflowUpdated = false;
    if (action === "update" || !existingWorkflow) {
      await upsertRepositoryFile({
        token: githubToken,
        owner,
        repo,
        path: workflowPath,
        content: templateContent,
        message:
          action === "init"
            ? "Initialize Atlas CI workflow"
            : "Update Atlas CI workflow",
        sha: existingWorkflow?.sha,
      });
      workflowUpdated = true;
    }

    await deleteRepositorySecret({
      token: githubToken,
      owner,
      repo,
      name: "ATLAS_API_KEY",
    });

    await setRepositoryActionsPermissions({
      token: githubToken,
      owner,
      repo,
    });
    await enableRepositoryWorkflows({
      token: githubToken,
      owner,
      repo,
    });

    return NextResponse.json({
      mode: action,
      packId,
      repository: `${owner}/${repo}`,
      workflowPath,
      workflowUpdated,
      secretsUpdated: true,
      actionsEnabled: true,
    });
  } catch (error) {
    return NextResponse.json(
      {
        error:
          error instanceof Error ? error.message : "Unable to synchronize CI workflow.",
      },
      { status: 502 }
    );
  }
}
