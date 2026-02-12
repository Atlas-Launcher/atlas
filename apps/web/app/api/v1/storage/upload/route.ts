import { NextResponse } from "next/server";

export async function PUT() {
  return NextResponse.json(
    {
      error:
        "Hub upload proxy is disabled. Use /api/v1/storage/presign and upload directly to storage.",
    },
    { status: 410 }
  );
}
