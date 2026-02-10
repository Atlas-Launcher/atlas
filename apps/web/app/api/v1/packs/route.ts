import { NextResponse } from "next/server";
import { and, desc, eq } from "drizzle-orm";

import { auth } from "@/auth";
import { db } from "@/lib/db";
import { accounts, packMembers, packs } from "@/lib/db/schema";
import { hasRole } from "@/lib/auth/roles";
import { createPackWithDefaults } from "@/lib/packs/create-pack";

function getAtlasHubUrl(request: Request) {
  return (
    process.env.ATLAS_HUB_URL?.trim() ??
    process.env.BETTER_AUTH_URL?.trim() ??
    new URL(request.url).origin
  );
}

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
  let existingAtlasToml: any = null;
  let parsed: { owner: string; repo: string } | null = null;
  let account: any = null;

  if (repoUrl) {
    const {
      parseGithubRepoUrl,
      checkAtlasTomlExists,
      configureRepoForAtlas,
    } = await import("@/lib/github/repo-config");
    const { getInstallationTokenForUser, GitHubAppNotInstalledError } = await import(
      "@/lib/github/app"
    );

    parsed = parseGithubRepoUrl(repoUrl);
    if (!parsed) {
      return NextResponse.json(
        { error: "Invalid GitHub repository URL." },
        { status: 400 }
      );
    }

    // Get the user's GitHub account
    [account] = await db
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
        },
      });

      if (!userResponse.ok) {
        return NextResponse.json(
          { error: "Failed to get GitHub user info." },
          { status: 500 }
        );
      }

      const userData = await userResponse.json();
      githubUsername = userData.login;
    } catch (error) {
      return NextResponse.json(
        { error: "Failed to get GitHub user info." },
        { status: 500 }
      );
    }

    // Check if atlas.toml exists
    try {
      existingAtlasToml = await checkAtlasTomlExists({ token: account.accessToken, owner: parsed.owner, repo: parsed.repo });
      if (!existingAtlasToml) {
        return NextResponse.json(
          { error: "Repository does not contain atlas.toml configuration." },
          { status: 400 }
        );
      }
    } catch (error) {
      return NextResponse.json(
        { error: "Repository does not contain atlas.toml configuration." },
        { status: 400 }
      );
    }
  }

  // Create the pack
  try {
    const pack = await createPackWithDefaults({
      name,
      description,
      repoUrl,
      slug,
      ownerId: session.user.id,
    });

    // If importing a repo, configure it for Atlas
    if (repoUrl && parsed && account && existingAtlasToml) {
      const { configureRepoForAtlas } = await import("@/lib/github/repo-config");

      // Configure the repo for Atlas
      try {
        await configureRepoForAtlas({
          token: account.accessToken,
          owner: parsed.owner,
          repo: parsed.repo,
          packId: pack.id,
          hubUrl: getAtlasHubUrl(request),
          existingAtlasToml,
        });
      } catch (error) {
        return NextResponse.json(
          { error: "Failed to configure repository for Atlas." },
          { status: 500 }
        );
      }
    }

    return NextResponse.json({ pack }, { status: 201 });
  } catch (error) {
    console.error("Failed to create pack:", error);
    return NextResponse.json(
      { error: "Failed to create pack." },
      { status: 500 }
    );
  }
}