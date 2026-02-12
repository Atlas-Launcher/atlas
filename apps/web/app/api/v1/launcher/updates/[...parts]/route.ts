import { NextResponse } from "next/server";

import {
  isDistributionArch,
  isDistributionChannel,
  isDistributionOs,
  resolveDownloadRedirect,
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

function toTauriPlatformKey(
  os: "windows" | "macos" | "linux",
  arch: "x64" | "arm64"
) {
  const tauriOs = os === "macos" ? "darwin" : os;
  const tauriArch = arch === "x64" ? "x86_64" : "aarch64";
  return `${tauriOs}-${tauriArch}`;
}

async function buildTauriResponse({
  release,
  os,
  arch,
  requestOrigin,
}: {
  release: NonNullable<Awaited<ReturnType<typeof resolveRelease>>>;
  os: "windows" | "macos" | "linux";
  arch: "x64" | "arm64";
  requestOrigin: string;
}) {
  const primaryAsset =
    release.assets.find((asset) => asset.kind === "installer") ??
    release.assets.find((asset) => asset.kind === "binary") ??
    null;
  const signatureAsset = release.assets.find((asset) => asset.kind === "signature") ?? null;

  if (!primaryAsset || !signatureAsset) {
    return null;
  }

  const signatureUrl = await resolveDownloadRedirect(signatureAsset.download_id);
  if (!signatureUrl) {
    return null;
  }

  const signatureResponse = await fetch(signatureUrl, { cache: "no-store" });
  if (!signatureResponse.ok) {
    return null;
  }
  const signature = (await signatureResponse.text()).trim();
  if (!signature) {
    return null;
  }

  const platformKey = toTauriPlatformKey(os, arch);

  return {
    version: release.version,
    notes: "",
    pub_date: release.published_at,
    platforms: {
      [platformKey]: {
        url: `${requestOrigin}/api/v1/download/${primaryAsset.download_id}`,
        signature,
      },
    },
  };
}

export async function GET(
  request: Request,
  { params }: { params: Promise<{ parts: string[] }> }
) {
  const { parts } = await params;
  let channel = "stable";
  let osInput = "";
  let archInput = "";

  if (parts.length === 2) {
    [osInput, archInput] = parts;
  } else if (parts.length === 3) {
    [channel, osInput, archInput] = parts;
  } else {
    return NextResponse.json({ error: "Invalid updater path." }, { status: 400 });
  }

  const os = normalizeUpdaterOs(osInput);
  const arch = normalizeUpdaterArch(archInput);
  if (!isDistributionOs(os) || !isDistributionArch(arch)) {
    return NextResponse.json({ error: "Invalid platform." }, { status: 400 });
  }

  channel = channel.trim().toLowerCase();
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

  const response = await buildTauriResponse({
    release,
    os,
    arch,
    requestOrigin: new URL(request.url).origin,
  });
  if (!response) {
    return NextResponse.json(
      { error: "Release is missing updater-compatible assets." },
      { status: 404 }
    );
  }

  return NextResponse.json(response, {
    headers: {
      "cache-control": "public, max-age=60, s-maxage=60",
    },
  });
}
