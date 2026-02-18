/**
 * GitHub App authentication utilities.
 *
 * This module provides functions to authenticate as a GitHub App and
 * generate installation access tokens for accessing repositories.
 */

import { SignJWT } from "jose";
import { createPrivateKey } from "crypto";

export class GitHubAppNotInstalledError extends Error {
    constructor(message: string = "GitHub App is not installed for this user.") {
        super(message);
        this.name = "GitHubAppNotInstalledError";
    }
}

export class GitHubAppNotConfiguredError extends Error {
    constructor() {
        super("GitHub App credentials are not configured on the server.");
        this.name = "GitHubAppNotConfiguredError";
    }
}

async function requestWithAppJwt<T>(url: string, init?: RequestInit): Promise<T> {
    const jwt = await generateAppJWT();
    const response = await fetch(url, {
        ...init,
        headers: {
            Authorization: `Bearer ${jwt}`,
            Accept: "application/vnd.github+json",
            ...(init?.headers ?? {}),
        },
    });

    if (!response.ok) {
        if (response.status === 404) {
            return null as T;
        }

        const body = await response.json().catch(() => ({}));
        throw new Error(body?.message ?? "GitHub App request failed");
    }

    return response.json();
}

/**
 * Generates a JWT for authenticating as the GitHub App.
 * The JWT is valid for 10 minutes (GitHub's maximum).
 */
async function generateAppJWT(): Promise<string> {
    const GITHUB_APP_ID = process.env.GITHUB_APP_ID;
    const GITHUB_APP_PRIVATE_KEY = process.env.GITHUB_APP_PRIVATE_KEY;

    if (!GITHUB_APP_ID || !GITHUB_APP_PRIVATE_KEY) {
        throw new GitHubAppNotConfiguredError();
    }

    // Handle newlines in the private key (env vars often escape \n)
    const privateKeyPem = GITHUB_APP_PRIVATE_KEY.replace(/\\n/g, "\n");

    // Use Node's crypto module which handles both PKCS#1 and PKCS#8 formats
    const privateKey = createPrivateKey(privateKeyPem);

    const now = Math.floor(Date.now() / 1000);

    const jwt = await new SignJWT({})
        .setProtectedHeader({ alg: "RS256" })
        .setIssuedAt(now - 60) // 60 seconds in the past to account for clock drift
        .setExpirationTime(now + 600) // 10 minutes
        .setIssuer(GITHUB_APP_ID)
        .sign(privateKey);

    return jwt;
}

/**
 * Finds the GitHub App installation for a given GitHub user ID.
 * Returns the installation ID if found, null otherwise.
 */
export async function findInstallationForUser(
    githubAccountId: string
): Promise<number | null> {
    return findInstallationForOwner(githubAccountId);
}

/**
 * Finds the GitHub App installation for a given owner login.
 * Supports both user and organization owners.
 */
export async function findInstallationForOwner(
    owner: string
): Promise<number | null> {
    const userInstallation = await requestWithAppJwt<{ id?: number } | null>(
        `https://api.github.com/users/${encodeURIComponent(owner)}/installation`
    );
    if (userInstallation?.id) {
        return userInstallation.id;
    }

    const orgInstallation = await requestWithAppJwt<{ id?: number } | null>(
        `https://api.github.com/orgs/${encodeURIComponent(owner)}/installation`
    );
    if (orgInstallation?.id) {
        return orgInstallation.id;
    }

    return null;
}

/**
 * Generates an installation access token for the given installation ID.
 * This token can be used to access repositories that the App has access to.
 */
export async function getInstallationAccessToken(
    installationId: number
): Promise<string> {
    const jwt = await generateAppJWT();

    const response = await fetch(
        `https://api.github.com/app/installations/${installationId}/access_tokens`,
        {
            method: "POST",
            headers: {
                Authorization: `Bearer ${jwt}`,
                Accept: "application/vnd.github+json",
            },
        }
    );

    if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body?.message ?? "Failed to generate installation access token");
    }

    const data = await response.json();
    return data.token;
}

/**
 * Gets an installation access token for a GitHub user.
 * This is a convenience function that combines findInstallationForUser
 * and getInstallationAccessToken.
 *
 * @param githubUsername The GitHub username to find the installation for.
 * @throws GitHubAppNotInstalledError if the App is not installed for the user.
 */
export async function getInstallationTokenForUser(
    githubUsername: string
): Promise<string> {
    return getInstallationTokenForOwner(githubUsername);
}

/**
 * Gets an installation access token for a GitHub owner (user or org).
 */
export async function getInstallationTokenForOwner(
    owner: string
): Promise<string> {
    const installationId = await findInstallationForOwner(owner);

    if (!installationId) {
        throw new GitHubAppNotInstalledError();
    }

    return getInstallationAccessToken(installationId);
}

/**
 * Lists repositories accessible to the GitHub App installation.
 */
export async function listInstallationRepos(
    installationToken: string,
    page: number = 1,
    perPage: number = 10
): Promise<{
    repos: Array<{
        name: string;
        full_name: string;
        html_url: string;
        clone_url: string;
        owner?: { login?: string };
    }>;
    totalCount: number;
    hasNextPage: boolean;
}> {
    const response = await fetch(
        `https://api.github.com/installation/repositories?per_page=${perPage}&page=${page}`,
        {
            headers: {
                Authorization: `Bearer ${installationToken}`,
                Accept: "application/vnd.github+json",
            },
        }
    );

    if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body?.message ?? "Failed to list installation repositories");
    }

    const data = await response.json();
    const repos = data.repositories ?? [];
    const totalCount = data.total_count ?? 0;

    return {
        repos: repos.map((repo: Record<string, unknown>) => ({
            name: repo.name,
            full_name: repo.full_name,
            html_url: repo.html_url,
            clone_url: repo.clone_url,
            owner: repo.owner ? { login: (repo.owner as { login?: string }).login } : undefined,
        })),
        totalCount,
        hasNextPage: page * perPage < totalCount,
    };
}
