import crypto from "crypto";

import type { StorageProviderId } from "@/lib/storage/types";

interface StorageTokenPayload {
  action: "upload" | "download";
  provider: StorageProviderId;
  key: string;
  exp: number;
}

function getSigningSecret() {
  const secret =
    process.env.STORAGE_UPLOAD_TOKEN_SECRET ??
    process.env.NEXTAUTH_SECRET ??
    process.env.BETTER_AUTH_SECRET ??
    process.env.AUTH_SECRET;

  if (!secret) {
    throw new Error(
      "Missing signing secret: set STORAGE_UPLOAD_TOKEN_SECRET (or NEXTAUTH_SECRET)."
    );
  }

  return secret;
}

function encodeBase64Url(value: string) {
  return Buffer.from(value).toString("base64url");
}

function decodeBase64Url(value: string) {
  return Buffer.from(value, "base64url").toString("utf8");
}

function sign(value: string) {
  return crypto.createHmac("sha256", getSigningSecret()).update(value).digest("base64url");
}

export function createStorageToken({
  action,
  provider,
  key,
  expiresInSeconds = 900,
}: {
  action: "upload" | "download";
  provider: StorageProviderId;
  key: string;
  expiresInSeconds?: number;
}) {
  const payload: StorageTokenPayload = {
    action,
    provider,
    key,
    exp: Math.floor(Date.now() / 1000) + expiresInSeconds,
  };

  const payloadSegment = encodeBase64Url(JSON.stringify(payload));
  const signatureSegment = sign(payloadSegment);
  return `${payloadSegment}.${signatureSegment}`;
}

export function verifyStorageToken(
  token: string,
  expectedAction: "upload" | "download"
): StorageTokenPayload {
  const [payloadSegment, signatureSegment] = token.split(".", 2);
  if (!payloadSegment || !signatureSegment) {
    throw new Error("Invalid storage token format");
  }

  const expectedSignature = sign(payloadSegment);
  if (
    signatureSegment.length !== expectedSignature.length ||
    !crypto.timingSafeEqual(
      Buffer.from(signatureSegment, "utf8"),
      Buffer.from(expectedSignature, "utf8")
    )
  ) {
    throw new Error("Invalid storage token signature");
  }

  const payload = JSON.parse(decodeBase64Url(payloadSegment)) as StorageTokenPayload;
  if (payload.action !== expectedAction) {
    throw new Error("Storage token action mismatch");
  }

  const now = Math.floor(Date.now() / 1000);
  if (payload.exp <= now) {
    throw new Error("Storage token expired");
  }

  return payload;
}
