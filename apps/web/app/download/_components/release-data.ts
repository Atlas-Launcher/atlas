import type {
  DistributionArch,
  DistributionProduct,
} from "@/lib/db/schema";
import { resolveRelease } from "@/lib/distribution";

import type { ProductPlatformTarget } from "@/app/download/_components/shared";

const DEFAULT_RELEASE_TARGETS: Array<{ os: "windows" | "macos" | "linux"; arch: DistributionArch }> = [
  { os: "windows", arch: "x64" },
  { os: "macos", arch: "arm64" },
  { os: "macos", arch: "x64" },
  { os: "linux", arch: "x64" },
  { os: "linux", arch: "arm64" },
];

export async function resolveLatestReleaseForProduct(
  product: DistributionProduct,
  targets: Array<{ os: "windows" | "macos" | "linux"; arch: DistributionArch }> = DEFAULT_RELEASE_TARGETS,
) {
  for (const target of targets) {
    const release = await resolveRelease({
      product,
      os: target.os,
      arch: target.arch,
      channel: "stable",
    });
    if (release) {
      return release;
    }
  }

  return null;
}

export async function resolvePlatformReleases(
  product: DistributionProduct,
  platformTargets: readonly ProductPlatformTarget[],
) {
  return Promise.all(
    platformTargets.flatMap((platform) =>
      platform.arches.map(async (arch) => ({
        key: `${platform.os}-${arch}`,
        os: platform.os,
        arch,
        release: await resolveRelease({
          product,
          os: platform.os,
          arch,
          channel: "stable",
        }),
      })),
    ),
  );
}
