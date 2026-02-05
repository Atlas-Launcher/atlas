import crypto from "crypto";

export function hashDeployToken(token: string) {
  return crypto.createHash("sha256").update(token).digest("hex");
}

export function generateDeployToken() {
  return crypto.randomBytes(32).toString("hex");
}
