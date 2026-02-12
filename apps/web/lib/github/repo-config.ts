/**
 * GitHub repository configuration utilities for Atlas.
 *
 * This module provides functions to configure repositories for Atlas CI,
 * including checking for and updating atlas.toml, and managing workflows.
 */
import sodium from "libsodium-wrappers";

import { rotateManagedPackDeployToken } from "@/lib/auth/deploy-tokens";

export type GithubContentFile = {
    sha: string;
    content: string;
    encoding: string;
};

export type GithubWorkflow = {
    id: number;
    state: string;
};

export type GithubWorkflowListResponse = {
    workflows?: GithubWorkflow[];
};

type GithubApiErrorPayload = {
    message?: string;
    documentation_url?: string;
};

type GithubActionsPublicKey = {
    key_id: string;
    key: string;
};

const MANAGED_PACK_DEPLOY_TOKEN_NAME = "github_actions_pack_deploy";

export class GithubApiError extends Error {
    status: number;
    url: string;
    payload: GithubApiErrorPayload;

    constructor({
        status,
        url,
        payload,
    }: {
        status: number;
        url: string;
        payload: GithubApiErrorPayload;
    }) {
        const details = payload.message ? `: ${payload.message}` : "";
        super(`${url} returned ${status}${details}`);
        this.name = "GithubApiError";
        this.status = status;
        this.url = url;
        this.payload = payload;
    }
}

function isGithubApiStatus(error: unknown, status: number) {
    return error instanceof GithubApiError && error.status === status;
}

function encodeGithubContentPath(path: string) {
    const normalized = path.trim().replace(/^\/+|\/+$/g, "");
    if (!normalized) {
        throw new Error("GitHub content path must not be empty.");
    }

    return normalized.split("/").map((segment) => encodeURIComponent(segment)).join("/");
}

function buildGithubContentsUrl({
    owner,
    repo,
    path,
}: {
    owner: string;
    repo: string;
    path: string;
}) {
    return `https://api.github.com/repos/${owner}/${repo}/contents/${encodeGithubContentPath(path)}`;
}

/**
 * Makes an authenticated request to the GitHub API.
 */
export async function githubRequest<T>(
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
        const payload = (await response.json().catch(() => ({}))) as GithubApiErrorPayload;
        throw new GithubApiError({
            status: response.status,
            url,
            payload,
        });
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

/**
 * Quotes a string value for TOML format.
 */
export function quoteTomlString(value: string) {
    return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

/**
 * Decodes base64-encoded content from GitHub API.
 */
export function decodeBase64Content(content: string) {
    return Buffer.from(content.replace(/\n/g, ""), "base64").toString("utf8");
}

/**
 * Updates or inserts a field in the [cli] section of a TOML file.
 */
export function upsertCliTomlField(toml: string, key: string, value: string) {
    const newline = toml.includes("\r\n") ? "\r\n" : "\n";
    const lines = toml.split(/\r?\n/);
    const hasTrailingNewline = /(?:\r?\n)$/.test(toml);
    const cliHeaderPattern = /^\s*\[cli\]\s*$/;
    const sectionPattern = /^\s*\[[^\]]+\]\s*$/;
    const fieldPattern = new RegExp(`^\\s*${key}\\s*=`);
    const replacement = `${key} = ${quoteTomlString(value)}`;

    const cliStart = lines.findIndex((line) => cliHeaderPattern.test(line));
    if (cliStart === -1) {
        if (lines.length && lines[lines.length - 1].trim().length !== 0) {
            lines.push("");
        }
        lines.push("[cli]");
        lines.push(replacement);
    } else {
        let cliEnd = lines.length;
        for (let i = cliStart + 1; i < lines.length; i += 1) {
            if (sectionPattern.test(lines[i])) {
                cliEnd = i;
                break;
            }
        }

        const fieldIndex = lines.findIndex(
            (line, index) => index > cliStart && index < cliEnd && fieldPattern.test(line)
        );

        if (fieldIndex >= 0) {
            lines[fieldIndex] = replacement;
        } else {
            lines.splice(cliStart + 1, 0, replacement);
        }
    }

    const output = lines.join(newline);
    return hasTrailingNewline ? `${output}${newline}` : output;
}

function sleep(ms: number) {
    return new Promise<void>((resolve) => {
        setTimeout(resolve, ms);
    });
}

/**
 * Parses a GitHub repository URL into owner and repo components.
 */
export function parseGithubRepoUrl(repoUrl: string): { owner: string; repo: string } | null {
    const trimmed = repoUrl.trim().replace(/\.git$/, "");

    // Handle https://github.com/owner/repo format
    const httpsMatch = trimmed.match(/github\.com\/([^\/]+)\/([^\/]+)/);
    if (httpsMatch) {
        return { owner: httpsMatch[1], repo: httpsMatch[2] };
    }

    // Handle git@github.com:owner/repo format
    const sshMatch = trimmed.match(/github\.com:([^\/]+)\/([^\/]+)/);
    if (sshMatch) {
        return { owner: sshMatch[1], repo: sshMatch[2] };
    }

    return null;
}

/**
 * Checks if an atlas.toml file exists in the repository.
 * Returns the file info if it exists, null otherwise.
 */
export async function checkAtlasTomlExists({
    token,
    owner,
    repo,
}: {
    token: string;
    owner: string;
    repo: string;
}): Promise<GithubContentFile | null> {
    try {
        const file = await githubRequest<GithubContentFile>(
            token,
            buildGithubContentsUrl({
                owner,
                repo,
                path: "atlas.toml",
            })
        );
        return file;
    } catch (error) {
        // Check if it's a 404 (file doesn't exist)
        if (isGithubApiStatus(error, 404)) {
            return null;
        }
        throw error;
    }
}

/**
 * Updates the atlas.toml file with pack_id and hub_url in the [cli] section.
 */
export async function updateAtlasTomlConfig({
    token,
    owner,
    repo,
    packId,
    hubUrl,
    existingFile,
}: {
    token: string;
    owner: string;
    repo: string;
    packId: string;
    hubUrl: string;
    existingFile: GithubContentFile;
}) {
    if (existingFile.encoding !== "base64") {
        throw new Error("Unexpected atlas.toml encoding from GitHub API.");
    }

    const currentToml = decodeBase64Content(existingFile.content);
    let updatedToml = upsertCliTomlField(currentToml, "pack_id", packId);
    updatedToml = upsertCliTomlField(updatedToml, "hub_url", hubUrl);

    if (updatedToml === currentToml) {
        return; // No changes needed
    }

    await githubRequest<null>(
        token,
        `https://api.github.com/repos/${owner}/${repo}/contents/atlas.toml`,
        {
            method: "PUT",
            body: JSON.stringify({
                message: "Configure Atlas Hub CLI defaults",
                content: Buffer.from(updatedToml, "utf8").toString("base64"),
                sha: existingFile.sha,
            }),
        }
    );
}

/**
 * Gets the workflow template content from the configured template repository.
 */
async function getWorkflowTemplate({
    token,
}: {
    token: string;
}): Promise<{ content: string; path: string }> {
    const templateRepoEnv = process.env.ATLAS_GITHUB_TEMPLATE_REPO?.trim();
    if (!templateRepoEnv) {
        throw new Error("ATLAS_GITHUB_TEMPLATE_REPO is not configured.");
    }

    const [templateOwner, templateRepo] = templateRepoEnv.split("/", 2);
    if (!templateOwner || !templateRepo) {
        throw new Error("ATLAS_GITHUB_TEMPLATE_REPO must be in the format 'owner/repo'.");
    }

    const workflowPath =
        process.env.ATLAS_CI_WORKFLOW_PATH?.trim() || ".github/workflows/atlas-build.yml";

    const file = await githubRequest<GithubContentFile>(
        token,
        buildGithubContentsUrl({
            owner: templateOwner,
            repo: templateRepo,
            path: workflowPath,
        })
    );

    if (file.encoding !== "base64") {
        throw new Error("Unexpected workflow file encoding from GitHub API.");
    }

    return {
        content: decodeBase64Content(file.content),
        path: workflowPath,
    };
}

/**
 * Ensures the atlas-build.yml workflow exists in the target repo.
 * Creates or updates the workflow file from the template.
 */
export async function ensureAtlasBuildWorkflow({
    token,
    owner,
    repo,
}: {
    token: string;
    owner: string;
    repo: string;
}) {
    const template = await getWorkflowTemplate({ token });
    const targetPath = ".github/workflows/atlas-build.yml";

    // Check if the workflow already exists
    let existingSha: string | null = null;
    try {
        const existing = await githubRequest<GithubContentFile>(
            token,
            buildGithubContentsUrl({
                owner,
                repo,
                path: targetPath,
            })
        );
        existingSha = existing.sha;
    } catch (error) {
        // File doesn't exist, that's fine
        if (!isGithubApiStatus(error, 404)) {
            throw error;
        }
    }

    // Create or update the workflow file
    await githubRequest<null>(
        token,
        buildGithubContentsUrl({
            owner,
            repo,
            path: targetPath,
        }),
        {
            method: "PUT",
            body: JSON.stringify({
                message: existingSha
                    ? "Update Atlas build workflow"
                    : "Add Atlas build workflow",
                content: Buffer.from(template.content, "utf8").toString("base64"),
                ...(existingSha ? { sha: existingSha } : {}),
            }),
        }
    );
}

/**
 * Sets repository Actions permissions to allow all actions.
 */
export async function setRepositoryActionsPermissions({
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

async function upsertRepositoryActionsSecret({
    token,
    owner,
    repo,
    secretName,
    secretValue,
}: {
    token: string;
    owner: string;
    repo: string;
    secretName: string;
    secretValue: string;
}) {
    const keyPayload = await githubRequest<GithubActionsPublicKey>(
        token,
        `https://api.github.com/repos/${owner}/${repo}/actions/secrets/public-key`
    );

    await sodium.ready;
    const publicKey = sodium.from_base64(
        keyPayload.key,
        sodium.base64_variants.ORIGINAL
    );
    const encrypted = sodium.crypto_box_seal(
        sodium.from_string(secretValue),
        publicKey
    );
    const encryptedValue = sodium.to_base64(
        encrypted,
        sodium.base64_variants.ORIGINAL
    );

    await githubRequest<null>(
        token,
        `https://api.github.com/repos/${owner}/${repo}/actions/secrets/${encodeURIComponent(secretName)}`,
        {
            method: "PUT",
            body: JSON.stringify({
                encrypted_value: encryptedValue,
                key_id: keyPayload.key_id,
            }),
        }
    );
}

async function ensureRepositoryAtlasSecrets({
    token,
    owner,
    repo,
    packId,
    hubUrl,
}: {
    token: string;
    owner: string;
    repo: string;
    packId: string;
    hubUrl: string;
}) {
    const packDeployToken = await rotateManagedPackDeployToken(
        packId,
        MANAGED_PACK_DEPLOY_TOKEN_NAME
    );

    await upsertRepositoryActionsSecret({
        token,
        owner,
        repo,
        secretName: "ATLAS_HUB_URL",
        secretValue: hubUrl,
    });
    await upsertRepositoryActionsSecret({
        token,
        owner,
        repo,
        secretName: "ATLAS_PACK_DEPLOY_TOKEN",
        secretValue: packDeployToken,
    });
}

/**
 * Lists workflows in a repository.
 */
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

/**
 * Enables all disabled workflows in a repository.
 */
export async function enableRepositoryWorkflows({
    token,
    owner,
    repo,
}: {
    token: string;
    owner: string;
    repo: string;
}) {
    for (let attempt = 0; attempt < 5; attempt += 1) {
        const workflows = await listRepositoryWorkflows({ token, owner, repo });
        if (workflows.length === 0) {
            if (attempt === 4) {
                // No workflows found after retries, this might be okay for imports
                return;
            }
            await sleep(750 * (attempt + 1));
            continue;
        }

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

        return;
    }
}

/**
 * Full configuration for an imported repository.
 * - Updates atlas.toml with pack_id and hub_url
 * - Adds/replaces atlas-build.yml workflow
 * - Creates/rotates repository deploy secrets
 * - Enables Actions and workflows
 */
export async function configureRepoForAtlas({
    token,
    owner,
    repo,
    packId,
    hubUrl,
    existingAtlasToml,
}: {
    token: string;
    owner: string;
    repo: string;
    packId: string;
    hubUrl: string;
    existingAtlasToml: GithubContentFile;
}) {
    // Update atlas.toml configuration
    await updateAtlasTomlConfig({
        token,
        owner,
        repo,
        packId,
        hubUrl,
        existingFile: existingAtlasToml,
    });

    // Add/replace the workflow
    await ensureAtlasBuildWorkflow({ token, owner, repo });

    // Ensure required repo secrets for pack deploy automation
    await ensureRepositoryAtlasSecrets({ token, owner, repo, packId, hubUrl });

    // Enable Actions
    await setRepositoryActionsPermissions({ token, owner, repo });
    await enableRepositoryWorkflows({ token, owner, repo });
}
