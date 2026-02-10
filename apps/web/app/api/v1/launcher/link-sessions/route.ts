import crypto from "crypto";
import { NextResponse } from "next/server";

import { db } from "@/lib/db";
import { launcherLinkSessions } from "@/lib/db/schema";

const LINK_SESSION_TTL_MS = 10 * 60 * 1000;
const CODE_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

function randomId(prefix: string, bytes: number) {
  return `${prefix}${crypto.randomBytes(bytes).toString("hex")}`;
}

function generateLinkCode() {
  let raw = "";
  for (let i = 0; i < 8; i += 1) {
    const index = crypto.randomInt(0, CODE_ALPHABET.length);
    raw += CODE_ALPHABET[index];
  }
  return `${raw.slice(0, 4)}-${raw.slice(4)}`;
}

export async function POST() {
  const linkSessionId = randomId("ls_", 16);
  const linkCode = generateLinkCode();
  const proof = randomId("proof_", 24);
  const expiresAt = new Date(Date.now() + LINK_SESSION_TTL_MS);

  await db.insert(launcherLinkSessions).values({
    id: linkSessionId,
    code: linkCode,
    proof,
    expiresAt,
  });

  return NextResponse.json({
    linkSessionId,
    linkCode,
    proof,
    expiresAt: expiresAt.toISOString(),
  });
}
