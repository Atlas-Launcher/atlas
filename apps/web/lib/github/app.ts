/**
 * GitHub App authentication utilities.
 *
 * This module provides functions to authenticate as a GitHub App and
 * generate installation access tokens for accessing repositories.
 */

import { SignJWT, importPKCS8 } from "jose";

const GITHUB_APP_ID = process.env.GITHUB_APP_ID;
const GITHUB_APP_PRIVATE_KEY = process.env.GITHUB_APP_PRIVATE_KEY;

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

/**
 * Generates a JWT for authenticating as the GitHub App.
 * The JWT is valid for 10 minutes (GitHub's maximum).
 */
async function generateAppJWT(): Promise<string> {
    if (!GITHUB_APP_ID || !GITHUB_APP_PRIVATE_KEY) {
        throw new GitHubAppNotConfiguredError();
    }

    // Handle newlines in the private key (env vars often escape \n)
    const privateKeyPem = GITHUB_APP_PRIVATE_KEY.replace(/\\n/g, "\n");
    const privateKey = await importPKCS8(privateKeyPem, "RS256");

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
    const jwt = await generateAppJWT();

    // First, try to find an installation for the user directly
    const response = await fetch(
        `https://api.github.com/users/${githubAccountId}/installation`,
        {
            headers: {
                Authorization: `Bearer ${jwt}`,
                Accept: "application/vnd.github+json",
            },
        }
    );

    if (response.status === 404) {
        return null;
    }

    if (!response.ok) {
        const body = await response.json().catch(() => ({}));
        throw new Error(body?.message ?? "Failed to find GitHub App installation");
    }

    const data = await response.json();
    return data.id ?? null;
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
    const installationId = await findInstallationForUser(githubUsername);

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
