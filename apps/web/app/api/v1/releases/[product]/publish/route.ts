import { and, eq } from "drizzle-orm";
import { NextResponse } from "next/server";

import { db } from "@/lib/db";
import {
  distributionArtifacts,
  distributionReleases,
  distributionReleasePlatforms,
  users,
  type DistributionArtifactKind,
} from "@/lib/db/schema";
import {
  isDistributionArch,
  isDistributionOs,
  isDistributionProduct,
  makeDownloadId,
  normalizeDistributionChannel,
} from "@/lib/distribution";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { encodeArtifactRef, isStorageProviderEnabled } from "@/lib/storage/harness";
import type { StorageProviderId } from "@/lib/storage/types";

const ARTIFACT_KINDS: DistributionArtifactKind[] = [
  "installer",
  "binary",
  "signature",
  "updater-manifest",
  "other",
];

type ParsedPublishArtifact = {
  kind: DistributionArtifactKind;
  filename: string;
  size: number;
  sha256: string;
  artifactRef: string;
};

function isArtifactKind(value: string): value is DistributionArtifactKind {
  return ARTIFACT_KINDS.includes(value as DistributionArtifactKind);
}

function normalizeSha256(value: unknown): string | null {
  const normalized = value?.toString().trim().toLowerCase() ?? "";
  if (!/^[a-f0-9]{64}$/.test(normalized)) {
    return null;
  }
  return normalized;
}

export async function POST(
  request: Request,
  { params }: { params: Promise<{ product: string }> }
) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  const [user] = await db
    .select({ role: users.role })
    .from(users)
    .where(eq(users.id, userId))
    .limit(1);

  if (!user || (user.role !== "admin" && user.role !== "creator")) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  const { product } = await params;
  if (!isDistributionProduct(product)) {
    return NextResponse.json({ error: "Invalid product." }, { status: 400 });
  }

  const body = await request.json().catch(() => null);
  const version = body?.version?.toString().trim();
  const channel = normalizeDistributionChannel(body?.channel?.toString());
  const os = body?.platform?.os?.toString().trim();
  const arch = body?.platform?.arch?.toString().trim();
  const notes = body?.notes?.toString() ?? "";
  const publishedAtRaw = body?.published_at?.toString() ?? body?.publishedAt?.toString();
  const publishedAt = publishedAtRaw ? new Date(publishedAtRaw) : new Date();

  if (!version || !isDistributionOs(os) || !isDistributionArch(arch)) {
    return NextResponse.json({ error: "Invalid version or platform." }, { status: 400 });
  }

  if (Number.isNaN(publishedAt.getTime())) {
    return NextResponse.json({ error: "Invalid published_at timestamp." }, { status: 400 });
  }

  const artifactsInput: unknown[] = Array.isArray(body?.artifacts) ? body.artifacts : [];
  if (!artifactsInput.length) {
    return NextResponse.json({ error: "At least one artifact is required." }, { status: 400 });
  }

  const artifacts = artifactsInput
    .map((entry: unknown): ParsedPublishArtifact | null => {
      const row = entry as Record<string, unknown>;
      const key = row?.key?.toString().trim() ?? "";
      const provider = (row?.provider?.toString().trim() as StorageProviderId | undefined) ?? "r2";
      const filename = row?.filename?.toString().trim() ?? "";
      const size = Number(row?.size ?? 0);
      const sha256 = normalizeSha256(row?.sha256);
      const kind = row?.kind?.toString().trim() ?? "";

      if (!key || !filename || !Number.isFinite(size) || size < 0 || !sha256 || !isArtifactKind(kind)) {
        return null;
      }

      if (!isStorageProviderEnabled(provider)) {
        return null;
      }

      return {
        kind,
        filename,
        size,
        sha256,
        artifactRef: encodeArtifactRef({ provider, key }),
      };
    })
    .filter((value): value is ParsedPublishArtifact => value !== null);

  if (!artifacts.length) {
    return NextResponse.json({ error: "No valid artifacts in payload." }, { status: 400 });
  }

  const [release] = await db
    .insert(distributionReleases)
    .values({
      product,
      version,
      channel,
      publishedAt,
      notes,
    })
    .onConflictDoUpdate({
      target: [distributionReleases.product, distributionReleases.version, distributionReleases.channel],
      set: {
        publishedAt,
        notes,
      },
    })
    .returning({
      id: distributionReleases.id,
      product: distributionReleases.product,
      version: distributionReleases.version,
      channel: distributionReleases.channel,
      publishedAt: distributionReleases.publishedAt,
    });

  const [existingPlatform] = await db
    .select({ id: distributionReleasePlatforms.id })
    .from(distributionReleasePlatforms)
    .where(
      and(
        eq(distributionReleasePlatforms.releaseId, release.id),
        eq(distributionReleasePlatforms.os, os),
        eq(distributionReleasePlatforms.arch, arch),
      ),
    )
    .limit(1);

  let platformId = existingPlatform?.id;

  if (!platformId) {
    const [insertedPlatform] = await db
      .insert(distributionReleasePlatforms)
      .values({
        releaseId: release.id,
        os,
        arch,
      })
      .returning({ id: distributionReleasePlatforms.id });
    platformId = insertedPlatform.id;
  }

  await db.delete(distributionArtifacts).where(eq(distributionArtifacts.platformId, platformId));

  const insertedAssets = await db
    .insert(distributionArtifacts)
    .values(
      artifacts.map((artifact) => ({
        platformId,
        kind: artifact.kind,
        filename: artifact.filename,
        size: Math.trunc(artifact.size),
        sha256: artifact.sha256,
        downloadId: makeDownloadId(),
        artifactRef: artifact.artifactRef,
      })),
    )
    .returning({
      kind: distributionArtifacts.kind,
      filename: distributionArtifacts.filename,
      size: distributionArtifacts.size,
      sha256: distributionArtifacts.sha256,
      downloadId: distributionArtifacts.downloadId,
    });

  return NextResponse.json(
    {
      product: release.product,
      version: release.version,
      channel: release.channel,
      published_at: release.publishedAt.toISOString(),
      platform: { os, arch },
      assets: insertedAssets.map((asset) => ({
        kind: asset.kind,
        filename: asset.filename,
        size: asset.size,
        sha256: asset.sha256,
        download_id: asset.downloadId,
      })),
    },
    { status: 201 },
  );
}
