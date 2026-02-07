import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { accounts, packMembers, packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
import { createPackWithDefaults } from "@/lib/packs/create-pack";



export async function GET(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const isAdmin = hasRole(session, ["admin"]);

  const result = isAdmin
    ? await db.select().from(packs).orderBy(packs.createdAt)
    : await db
      .select({
        id: packs.id,
        name: packs.name,
        slug: packs.slug,
        description: packs.description,
        repoUrl: packs.repoUrl,
        createdAt: packs.createdAt,
        updatedAt: packs.updatedAt,
      })
      .from(packMembers)
      .innerJoin(packs, eq(packMembers.packId, packs.id))
      .where(eq(packMembers.userId, session.user.id));

  return NextResponse.json({ packs: result });
}

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const body = await request.json();
  const name = body?.name?.toString().trim();
  const description = body?.description?.toString().trim();
  const repoUrl = body?.repoUrl?.toString().trim();
  const slug = body?.slug?.toString().trim();

  if (!name) {
    return NextResponse.json({ error: "Name is required" }, { status: 400 });
  }

  // If importing a repo, validate it has atlas.toml and configure it
  if (repoUrl) {
    const {
      parseGithubRepoUrl,
      checkAtlasTomlExists,
      configureRepoForAtlas,
    } = await import("@/lib/github/repo-config");
    const { getInstallationTokenForUser, GitHubAppNotInstalledError } = await import(
      "@/lib/github/app"
    );

    const parsed = parseGithubRepoUrl(repoUrl);
    if (!parsed) {
      return NextResponse.json(
        { error: "Invalid GitHub repository URL." },
        { status: 400 }
      );
    }

    // Get the user's GitHub account
    const [account] = await db
      .select()
      .from(accounts)
      .where(
        and(eq(accounts.userId, session.user.id), eq(accounts.providerId, "github"))
      )
      .orderBy(desc(accounts.updatedAt))
      .limit(1);

    if (!account?.accessToken) {
      return NextResponse.json(
        { error: "No GitHub account linked." },
        { status: 404 }
      );
    }

    // Get the username from the user's token
    let githubUsername: string;
    try {
      const userResponse = await fetch("https://api.github.com/user", {
        headers: {
          Authorization: `Bearer ${account.accessToken}`,
          Accept: "application/vnd.github+json",
        },
      });
      if (!userResponse.ok) {
        throw new Error("Failed to get GitHub user info");
      }
      const userData = await userResponse.json();
      githubUsername = userData.login;
    } catch {
      return NextResponse.json(
        { error: "Unable to verify GitHub account. Please re-link your GitHub account." },
        { status: 400 }
      );
    }

    // Get installation token for repo access
    let installationToken: string;
    try {
      installationToken = await getInstallationTokenForUser(githubUsername);
    } catch (error) {
      if (error instanceof GitHubAppNotInstalledError) {
        const appSlug =
          process.env.GITHUB_APP_SLUG ||
          process.env.NEXT_PUBLIC_GITHUB_APP_SLUG ||
          "atlas-launcher";
        return NextResponse.json(
          {
            error:
              "The Atlas Launcher GitHub App is not installed on your account. Install it to import repositories.",
            code: "GITHUB_APP_NOT_INSTALLED",
            installUrl: `https://github.com/apps/${appSlug}/installations/new`,
          },
          { status: 403 }
        );
      }
      throw error;
    }

    // Check if atlas.toml exists
    const atlasToml = await checkAtlasTomlExists({
      token: installationToken,
      owner: parsed.owner,
      repo: parsed.repo,
    });

    if (!atlasToml) {
      return NextResponse.json(
        {
          error:
            "This repository does not contain an atlas.toml file. Please use the 'New GitHub Repository' option to create a properly configured pack repository.",
          code: "MISSING_ATLAS_TOML",
        },
        { status: 400 }
      );
    }

    // Create the pack first so we have the ID
    const created = await createPackWithDefaults({
      ownerId: session.user.id,
      name,
      description,
      repoUrl,
      slug,
    });

    // Configure the repository
    const hubUrl =
      process.env.ATLAS_HUB_URL?.trim() ??
      process.env.BETTER_AUTH_URL?.trim() ??
      new URL(request.url).origin;

    try {
      await configureRepoForAtlas({
        token: installationToken,
        owner: parsed.owner,
        repo: parsed.repo,
        packId: created.id,
        hubUrl,
        existingAtlasToml: atlasToml,
      });
    } catch (error) {
      // Log but don't fail - the pack was created, repo config can be retried
      console.error("Failed to configure repository:", error);
    }

    return NextResponse.json({ pack: created }, { status: 201 });
  }

  // Non-import flow (no repoUrl) - just create the pack
  const created = await createPackWithDefaults({
    ownerId: session.user.id,
    name,
    description,
    repoUrl,
    slug,
  });

  return NextResponse.json({ pack: created }, { status: 201 });
}
