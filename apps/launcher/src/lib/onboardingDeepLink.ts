import type { AtlasPackChannel } from "@/types/settings";

export type OnboardingIntentSource = "invite";

export interface OnboardingIntent {
  source: OnboardingIntentSource;
  packId: string;
  channel: AtlasPackChannel;
  createdAt: string;
}

function normalizeChannel(value: string | null): AtlasPackChannel | null {
  if (value === "dev" || value === "beta" || value === "production") {
    return value;
  }
  return null;
}

function isOnboardingTarget(url: URL): boolean {
  const host = url.hostname.trim().toLowerCase();
  const path = url.pathname.trim().toLowerCase();
  return host === "onboarding" || path === "/onboarding";
}

export function parseOnboardingDeepLink(rawUrl: string): OnboardingIntent | null {
  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    return null;
  }

  if (parsed.protocol !== "atlas:") {
    return null;
  }
  if (!isOnboardingTarget(parsed)) {
    return null;
  }

  const source = parsed.searchParams.get("source")?.trim().toLowerCase();
  const packId = parsed.searchParams.get("packId")?.trim();
  const channel = normalizeChannel(parsed.searchParams.get("channel")?.trim().toLowerCase() ?? null);

  if (source !== "invite" || !packId || !channel) {
    return null;
  }

  return {
    source: "invite",
    packId,
    channel,
    createdAt: new Date().toISOString(),
  };
}
