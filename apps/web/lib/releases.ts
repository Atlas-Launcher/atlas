export type ReleaseAsset = {
  id?: number;
  name: string;
  browser_download_url: string;
  size: number;
  content_type: string;
};

export type GitHubRelease = {
  tag_name: string;
  name: string | null;
  html_url: string;
  published_at?: string;
  created_at?: string;
  prerelease?: boolean;
  draft?: boolean;
  assets: ReleaseAsset[];
};

const DEFAULT_REVALIDATE_SECONDS = 300;

export function getReleaseRepo(): string | null {
  return (
    process.env.ATLAS_RELEASE_REPO ??
    process.env.NEXT_PUBLIC_ATLAS_RELEASE_REPO ??
    process.env.atlas_release_repo ??
    null
  );
}

export function getAuthHeaders(): HeadersInit {
  const token =
    process.env.ATLAS_GITHUB_TOKEN ?? process.env.atlas_github_token ?? process.env.GITHUB_TOKEN;
  if (!token) {
    return {};
  }

  return {
    Authorization: `Bearer ${token}`,
  };
}

async function fetchReleases(repo: string): Promise<GitHubRelease[]> {
  const response = await fetch(`https://api.github.com/repos/${repo}/releases?per_page=100`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "atlas-hub-downloads",
      ...getAuthHeaders(),
    },
    next: { revalidate: DEFAULT_REVALIDATE_SECONDS },
  });

  if (!response.ok) {
    return [];
  }

  const data = (await response.json()) as GitHubRelease[];
  return Array.isArray(data) ? data : [];
}

export async function getReleaseByTag(tag: string): Promise<GitHubRelease | null> {
  const repo = getReleaseRepo();
  if (!repo || !tag) {
    return null;
  }

  const response = await fetch(`https://api.github.com/repos/${repo}/releases/tags/${tag}`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "atlas-hub-downloads",
      ...getAuthHeaders(),
    },
    next: { revalidate: DEFAULT_REVALIDATE_SECONDS },
  });

  if (!response.ok) {
    return null;
  }

  const data = (await response.json()) as GitHubRelease;
  return data ?? null;
}

export async function getLatestRelease(tagPrefix: string): Promise<GitHubRelease | null> {
  const repo = getReleaseRepo();
  if (!repo) {
    return null;
  }

  const releases = await fetchReleases(repo);
  const matching = releases.filter(
    (release) => release.tag_name?.startsWith(tagPrefix) && !release.draft,
  );

  if (matching.length === 0) {
    return null;
  }

  const stable = matching.filter((release) => !release.prerelease);
  const candidates = stable.length ? stable : matching;

  const sorted = candidates.sort((a, b) => {
    const aTime = new Date(a.published_at ?? a.created_at ?? 0).getTime();
    const bTime = new Date(b.published_at ?? b.created_at ?? 0).getTime();
    return bTime - aTime;
  });

  return sorted[0] ?? null;
}
