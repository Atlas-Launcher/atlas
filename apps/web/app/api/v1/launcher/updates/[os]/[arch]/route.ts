import { NextResponse } from "next/server";

import { isDistributionArch, isDistributionOs, resolveRelease } from "@/lib/distribution";

function buildTauriResponse(release: NonNullable<Awaited<ReturnType<typeof resolveRelease>>>) {
  const primaryAsset =
    release.assets.find((asset) => asset.kind === "installer") ??
    release.assets.find((asset) => asset.kind === "binary") ??
    null;
  const signatureAsset = release.assets.find((asset) => asset.kind === "signature") ?? null;

  if (!primaryAsset || !signatureAsset) {
    return null;
  }

  const platformKey = `${release.platform.os}-${release.platform.arch}`;

  return {
    version: release.version,
    notes: "",
    pub_date: release.published_at,
    platforms: {
      [platformKey]: {
        url: `/api/v1/download/${primaryAsset.download_id}`,
        signature: `/api/v1/download/${signatureAsset.download_id}`,
      },
    },
  };
}

export async function GET(
  _request: Request,
  { params }: { params: Promise<{ os: string; arch: string }> }
) {
  const { os, arch } = await params;

  if (!isDistributionOs(os) || !isDistributionArch(arch)) {
    return NextResponse.json({ error: "Invalid platform." }, { status: 400 });
  }

  const release = await resolveRelease({
    product: "launcher",
    os,
    arch,
    channel: "stable",
  });

  if (!release) {
    return NextResponse.json({ error: "Release not found." }, { status: 404 });
  }

  const response = buildTauriResponse(release);
  if (!response) {
    return NextResponse.json(
      { error: "Release is missing updater-compatible assets." },
      { status: 404 },
    );
  }

  return NextResponse.json(response, {
    headers: {
      "cache-control": "public, max-age=60, s-maxage=60",
    },
  });
}
