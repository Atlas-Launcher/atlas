export function toRepositoryWebUrl(repoUrl?: string | null): string | null {
  const value = repoUrl?.trim();
  if (!value) {
    return null;
  }

  const sshMatch = value.match(
    /^(?:ssh:\/\/)?git@github\.com[:/]([^/\s]+)\/([^/\s]+?)(?:\.git)?\/?$/i
  );
  if (sshMatch) {
    const owner = sshMatch[1];
    const repo = sshMatch[2];
    return `https://github.com/${owner}/${repo}`;
  }

  const httpsValue = value.startsWith("git+https://")
    ? value.replace(/^git\+/, "")
    : value;
  try {
    const parsed = new URL(httpsValue);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return null;
    }

    parsed.hash = "";
    parsed.search = "";
    parsed.pathname = parsed.pathname.replace(/\/+$/, "");
    parsed.pathname = parsed.pathname.replace(/\.git$/i, "");
    if (!parsed.pathname || parsed.pathname === "/") {
      return null;
    }
    return parsed.toString().replace(/\/$/, "");
  } catch {
    return null;
  }
}

export function toRepositoryDisplayLabel(repoUrl?: string | null): string {
  const normalized = toRepositoryWebUrl(repoUrl);
  if (!normalized) {
    return "No repository linked";
  }

  try {
    const parsed = new URL(normalized);
    return `${parsed.hostname}${parsed.pathname}`.replace(/\/+$/, "");
  } catch {
    return normalized;
  }
}

export function toGithubCommitUrl(
  repoUrl?: string | null,
  commitHash?: string | null
): string | null {
  const normalizedRepo = toRepositoryWebUrl(repoUrl);
  const hash = commitHash?.trim();
  if (!normalizedRepo || !hash) {
    return null;
  }

  try {
    const parsed = new URL(normalizedRepo);
    if (!/^(www\.)?github\.com$/i.test(parsed.hostname)) {
      return null;
    }

    return `${normalizedRepo}/commit/${encodeURIComponent(hash)}`;
  } catch {
    return null;
  }
}

export function toRepositorySlug(repoUrl?: string | null): string | null {
  const normalizedRepo = toRepositoryWebUrl(repoUrl);
  if (!normalizedRepo) {
    return null;
  }

  try {
    const parsed = new URL(normalizedRepo);
    const segments = parsed.pathname.split("/").filter(Boolean);
    if (segments.length < 2) {
      return null;
    }
    return `${segments[0]}/${segments[1]}`.toLowerCase();
  } catch {
    return null;
  }
}
