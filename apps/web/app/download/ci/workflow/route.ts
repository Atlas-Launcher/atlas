import { NextResponse } from "next/server";

type GithubContentFile = {
  content?: string;
  encoding?: string;
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

function getWorkflowPath() {
  const value = process.env.ATLAS_CI_WORKFLOW_PATH?.trim();
  return value && value.length > 0 ? value : ".github/workflows/atlas-build.yml";
}

function encodeGithubContentPath(path: string) {
  const normalized = path.trim().replace(/^\/+|\/+$/g, "");
  if (!normalized) {
    throw new Error("ATLAS_CI_WORKFLOW_PATH must not be empty.");
  }

  return normalized.split("/").map((segment) => encodeURIComponent(segment)).join("/");
}

function getGithubAuthHeaders(): HeadersInit {
  const token = process.env.ATLAS_GITHUB_TOKEN?.trim();
  if (!token) {
    return {};
  }

  return {
    Authorization: `Bearer ${token}`,
  };
}

export async function GET() {
  let templateRepo: { owner: string; repo: string };
  try {
    templateRepo = getTemplateRepo();
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "Template repository is not configured." },
      { status: 500 }
    );
  }

  const workflowPath = getWorkflowPath();
  const endpoint = `https://api.github.com/repos/${templateRepo.owner}/${templateRepo.repo}/contents/${encodeGithubContentPath(workflowPath)}`;

  const response = await fetch(endpoint, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "atlas-hub-downloads",
      ...getGithubAuthHeaders(),
    },
    next: { revalidate: 300 },
  });

  if (response.status === 404) {
    return NextResponse.json(
      { error: "Template CI workflow was not found." },
      { status: 404 }
    );
  }

  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    return NextResponse.json(
      { error: body?.message ?? "Failed to fetch workflow template from GitHub." },
      { status: 502 }
    );
  }

  const payload = (await response.json()) as GithubContentFile;
  if (!payload.content || payload.encoding !== "base64") {
    return NextResponse.json(
      { error: "Template workflow file had an unsupported encoding." },
      { status: 502 }
    );
  }

  const content = Buffer.from(payload.content.replace(/\n/g, ""), "base64").toString("utf8");
  const filename = workflowPath.split("/").filter(Boolean).at(-1) ?? "atlas-build.yml";

  return new NextResponse(content, {
    headers: {
      "content-type": "text/yaml; charset=utf-8",
      "cache-control": "public, max-age=300",
      "content-disposition": `attachment; filename="${filename}"`,
      "x-atlas-workflow-path": workflowPath,
    },
  });
}
