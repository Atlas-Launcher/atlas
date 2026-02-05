import { NextResponse } from "next/server";

import { auth } from "@/auth";
import { hasRole } from "@/lib/auth/roles";
import {
  createPresignedDownloadUrl,
  createPresignedUploadUrl,
} from "@/lib/storage/r2";

export async function POST(request: Request) {
  const session = await auth.api.getSession({ headers: request.headers });

  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  if (!hasRole(session, ["admin", "creator"])) {
    return NextResponse.json({ error: "Forbidden" }, { status: 403 });
  }

  try {
    const body = await request.json();
    const key = body?.key?.toString();
    const contentType = body?.contentType?.toString();
    const action = body?.action?.toString() ?? "upload";

    if (!key) {
      return NextResponse.json({ error: "Key is required" }, { status: 400 });
    }

    if (action === "download") {
      const url = await createPresignedDownloadUrl({ key });
      return NextResponse.json({ url, key });
    }

    const url = await createPresignedUploadUrl({ key, contentType });
    return NextResponse.json({ url, key });
  } catch (error) {
    console.error("Presign error", error);
    return NextResponse.json(
      { error: "Unable to create presigned URL" },
      { status: 500 }
    );
  }
}
