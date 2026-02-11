import { NextResponse } from "next/server";

import {
  isDistributionArch,
  isDistributionOs,
  isDistributionProduct,
  normalizeDistributionChannel,
  resolveRelease,
} from "@/lib/distribution";

export async function GET(
  request: Request,
  { params }: { params: Promise<{ product: string; os: string; arch: string }> }
) {
  const { product, os, arch } = await params;

  if (!isDistributionProduct(product) || !isDistributionOs(os) || !isDistributionArch(arch)) {
    return NextResponse.json({ error: "Invalid product or platform." }, { status: 400 });
  }

  const url = new URL(request.url);
  const channel = normalizeDistributionChannel(url.searchParams.get("channel"));
  const release = await resolveRelease({ product, os, arch, channel });

  if (!release) {
    return NextResponse.json({ error: "Release not found." }, { status: 404 });
  }

  return NextResponse.json(release, {
    headers: {
      "cache-control": "public, max-age=60, s-maxage=60",
    },
  });
}
