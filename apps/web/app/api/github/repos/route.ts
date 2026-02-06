import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";
import sodium from "tweetsodium";

import { auth } from "@/auth";
import { hasRole } from "@/lib/auth/roles";
import { db } from "@/lib/db";
import { accounts, packs } from "@/lib/db/schema";
import { createPackWithDefaults } from "@/lib/packs/create-pack";

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

type GithubRepoPublicKey = {
  key_id: string;
  key: string;
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

function defaultKeyName(packId: string) {
  return `Pack ${packId.slice(0, 8)} deploy key`;
}

function getAtlasHubUrl(request: Request) {
  return (
    process.env.ATLAS_HUB_URL?.trim() ??
    process.env.BETTER_AUTH_URL?.trim() ??
    new URL(request.url).origin
  );
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

  if (response.status === 204) {
    return null as T;
  }

  const text = await response.text();
  if (!text) {
    return null as T;
  }

  return JSON.parse(text) as T;
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

function encryptSecret(secretValue: string, base64PublicKey: string) {
  const messageBytes = Buffer.from(secretValue);
  const publicKeyBytes = Buffer.from(base64PublicKey, "base64");
  const encryptedBytes = sodium.seal(messageBytes, publicKeyBytes);
  return Buffer.from(encryptedBytes).toString("base64");
}

async function setRepositorySecret({
  token,
  owner,
  repo,
  name,
  value,
}: {
  token: string;
  owner: string;
  repo: string;
  name: string;
  value: string;
}) {
  const publicKey = await githubRequest<GithubRepoPublicKey>(
    token,
    `https://api.github.com/repos/${owner}/${repo}/actions/secrets/public-key`
  );

  const encryptedValue = encryptSecret(value, publicKey.key);
  await githubRequest<null>(
    token,
    `https://api.github.com/repos/${owner}/${repo}/actions/secrets/${encodeURIComponent(name)}`,
    {
      method: "PUT",
      body: JSON.stringify({
        encrypted_value: encryptedValue,
        key_id: publicKey.key_id,
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

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const token = await getGithubToken(session.user.id);
  if (!token) {
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

  const [user, orgs] = await Promise.all([
    githubRequest<GithubOwner>(token, "https://api.github.com/user"),
    githubRequest<GithubOwner[]>(token, "https://api.github.com/user/orgs?per_page=100"),
  ]);

  const ownerIsUser = owner.toLowerCase() === user.login.toLowerCase();
  const ownerIsOrg = orgs.some((org) => org.login.toLowerCase() === owner.toLowerCase());

  if (!ownerIsUser && !ownerIsOrg) {
    return NextResponse.json(
      { error: "Selected owner is not available to this account." },
      { status: 400 }
    );
  }

  let createdPackId: string | null = null;
  let createdApiKeyId: string | null = null;
  let createdRepo: GithubRepo | null = null;

  try {
    const packName = name.replace(/[-_]+/g, " ").trim() || name;
    const createdPack = await createPackWithDefaults({
      ownerId: session.user.id,
      name: packName,
    });
    createdPackId = createdPack.id;

    const repo = await createRepositoryFromTemplate({
      token,
      owner,
      name,
      description,
      visibility,
    });
    createdRepo = repo;

    const apiKeyRecord = await auth.api.createApiKey({
      headers: request.headers,
      body: {
        name: defaultKeyName(createdPack.id),
        metadata: { packId: createdPack.id, type: "deploy" },
      },
    });

    const atlasApiKey = apiKeyRecord?.key?.toString().trim();
    if (!atlasApiKey) {
      throw new Error("Failed to create Atlas API key.");
    }
    createdApiKeyId =
      typeof apiKeyRecord?.id === "string" ? apiKeyRecord.id : null;

    const resolvedOwner = repo?.owner?.login ?? owner;
    const hubUrl = getAtlasHubUrl(request);

    await Promise.all([
      setRepositorySecret({
        token,
        owner: resolvedOwner,
        repo: repo.name,
        name: "ATLAS_API_KEY",
        value: atlasApiKey,
      }),
      setRepositorySecret({
        token,
        owner: resolvedOwner,
        repo: repo.name,
        name: "ATLAS_DEPLOY_KEY",
        value: atlasApiKey,
      }),
      setRepositorySecret({
        token,
        owner: resolvedOwner,
        repo: repo.name,
        name: "ATLAS_PACK_ID",
        value: createdPack.id,
      }),
      setRepositorySecret({
        token,
        owner: resolvedOwner,
        repo: repo.name,
        name: "ATLAS_HUB_URL",
        value: hubUrl,
      }),
    ]);

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
    if (createdApiKeyId) {
      await auth.api
        .deleteApiKey({
          headers: request.headers,
          body: { keyId: createdApiKeyId },
        })
        .catch(() => null);
    }

    if (createdRepo) {
      const repoOwner = createdRepo?.owner?.login ?? owner;
      if (repoOwner) {
        await deleteRepository({
          token,
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
            ? error.message
            : "Unable to create GitHub repository.",
      },
      { status: 502 }
    );
  }
}
