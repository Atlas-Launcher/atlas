import { NextResponse } from "next/server";
import { getAuthenticatedUserId } from "@/lib/auth/request-user";
import { syncMojangProfile } from "@/lib/auth/mojang";

export async function POST(request: Request) {
  const userId = await getAuthenticatedUserId(request);
  if (!userId) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  try {
    const profile = await syncMojangProfile(userId);
    return NextResponse.json(profile);
  } catch (error: unknown) {
    const message =
      error instanceof Error ? error.message : "Failed to sync Mojang profile.";
    return NextResponse.json({ error: message }, { status: 500 });
  }
}
