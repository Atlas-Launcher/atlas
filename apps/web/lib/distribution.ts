import { and, desc, eq } from "drizzle-orm";

import { db } from "@/lib/db";
import {
  distributionArtifacts,
  distributionReleases,
  distributionReleasePlatforms,
  type DistributionArtifactKind,
  type DistributionArch,
  type DistributionChannel,
  type DistributionOs,
  type DistributionProduct,
} from "@/lib/db/schema";
import { createDownloadUrlForArtifactRef, decodeArtifactRef } from "@/lib/storage/harness";

export const DISTRIBUTION_PRODUCTS = ["launcher", "cli", "runner", "runnerd"] as const;
export const DISTRIBUTION_OSES = ["windows", "macos", "linux"] as const;
export const DISTRIBUTION_ARCHES = ["x64", "arm64"] as const;
export const DISTRIBUTION_CHANNELS = ["stable", "beta", "dev"] as const;

export type DistributionReleaseResponse = {
  product: DistributionProduct;
  version: string;
  channel: DistributionChannel;
  published_at: string;
  platform: {
    os: DistributionOs;
    arch: DistributionArch;
  };
  assets: Array<{
    kind: DistributionArtifactKind;
    filename: string;
    size: number;
    sha256: string;
    download_id: string;
  }>;
};

export function isDistributionProduct(value: string): value is DistributionProduct {
  return (DISTRIBUTION_PRODUCTS as readonly string[]).includes(value);
}

export function isDistributionOs(value: string): value is DistributionOs {
  return (DISTRIBUTION_OSES as readonly string[]).includes(value);
}

export function isDistributionArch(value: string): value is DistributionArch {
  return (DISTRIBUTION_ARCHES as readonly string[]).includes(value);
}

export function normalizeDistributionChannel(value: string | null | undefined): DistributionChannel {
  const normalized = value?.trim().toLowerCase();
  if (normalized && (DISTRIBUTION_CHANNELS as readonly string[]).includes(normalized)) {
    return normalized as DistributionChannel;
  }
  return "stable";
}

export function isDistributionChannel(value: string): value is DistributionChannel {
  return (DISTRIBUTION_CHANNELS as readonly string[]).includes(value);
}

export function makeDownloadId() {
  const value = crypto.randomUUID().replace(/-/g, "");
  return `dl_${value}`;
}

export async function resolveRelease(params: {
  product: DistributionProduct;
  os: DistributionOs;
  arch: DistributionArch;
  version?: string;
  channel?: DistributionChannel;
}): Promise<DistributionReleaseResponse | null> {
  const channel = params.channel ?? "stable";

  const [release] = await db
    .select({
      id: distributionReleases.id,
      product: distributionReleases.product,
      version: distributionReleases.version,
      channel: distributionReleases.channel,
      publishedAt: distributionReleases.publishedAt,
    })
    .from(distributionReleases)
    .where(
      params.version
        ? and(
            eq(distributionReleases.product, params.product),
            eq(distributionReleases.version, params.version),
            eq(distributionReleases.channel, channel),
          )
        : and(eq(distributionReleases.product, params.product), eq(distributionReleases.channel, channel)),
    )
    .orderBy(desc(distributionReleases.publishedAt), desc(distributionReleases.createdAt))
    .limit(1);

  if (!release) {
    return null;
  }

  const [platform] = await db
    .select({
      id: distributionReleasePlatforms.id,
      os: distributionReleasePlatforms.os,
      arch: distributionReleasePlatforms.arch,
    })
    .from(distributionReleasePlatforms)
    .where(
      and(
        eq(distributionReleasePlatforms.releaseId, release.id),
        eq(distributionReleasePlatforms.os, params.os),
        eq(distributionReleasePlatforms.arch, params.arch),
      ),
    )
    .limit(1);

  if (!platform) {
    return null;
  }

  const assets = await db
    .select({
      kind: distributionArtifacts.kind,
      filename: distributionArtifacts.filename,
      size: distributionArtifacts.size,
      sha256: distributionArtifacts.sha256,
      downloadId: distributionArtifacts.downloadId,
    })
    .from(distributionArtifacts)
    .where(eq(distributionArtifacts.platformId, platform.id));

  return {
    product: release.product,
    version: release.version,
    channel: release.channel,
    published_at: release.publishedAt.toISOString(),
    platform: {
      os: platform.os,
      arch: platform.arch,
    },
    assets: assets
      .map((asset) => ({
        kind: asset.kind,
        filename: asset.filename,
        size: asset.size,
        sha256: asset.sha256,
        download_id: asset.downloadId,
      }))
      .sort((a, b) => a.kind.localeCompare(b.kind) || a.filename.localeCompare(b.filename)),
  };
}

export async function resolveDownloadRedirect(downloadId: string): Promise<string | null> {
  const [artifact] = await db
    .select({ artifactRef: distributionArtifacts.artifactRef })
    .from(distributionArtifacts)
    .where(eq(distributionArtifacts.downloadId, downloadId))
    .limit(1);

  if (!artifact) {
    return null;
  }

  const artifactRef = decodeArtifactRef(artifact.artifactRef);
  return createDownloadUrlForArtifactRef(artifactRef);
}
