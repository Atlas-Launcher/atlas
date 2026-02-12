import { NextResponse } from "next/server";

import {
  isDistributionArch,
  isDistributionChannel,
  isDistributionOs,
  resolveRelease,
} from "@/lib/distribution";

function normalizeUpdaterOs(value: string) {
  const normalized = value.trim().toLowerCase();
  if (normalized === "win32") return "windows";
  if (normalized === "darwin" || normalized === "osx") return "macos";
  return normalized;
}

function normalizeUpdaterArch(value: string) {
  const normalized = value.trim().toLowerCase();
  if (normalized === "x86_64" || normalized === "amd64") return "x64";
  if (normalized === "aarch64") return "arm64";
  return normalized;
}

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
  { params }: { params: Promise<{ channel: string; os: string; arch: string }> }
) {
  const { channel: channelParam, os: osInput, arch: archInput } = await params;
  const os = normalizeUpdaterOs(osInput);
  const arch = normalizeUpdaterArch(archInput);

  if (!isDistributionOs(os) || !isDistributionArch(arch)) {
    return NextResponse.json({ error: "Invalid platform." }, { status: 400 });
  }

  const channel = channelParam.trim().toLowerCase();
  if (!isDistributionChannel(channel)) {
    return NextResponse.json({ error: "Invalid channel." }, { status: 400 });
  }

  const release = await resolveRelease({
    product: "launcher",
    os,
    arch,
    channel,
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
