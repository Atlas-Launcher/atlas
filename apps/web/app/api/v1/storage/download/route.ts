import { NextResponse } from "next/server";

export async function GET() {
  return NextResponse.json(
    {
      error:
        "Hub download proxy is disabled. Use /api/v1/storage/presign?action=download and direct storage URLs.",
    },
    { status: 410 }
  );
}
