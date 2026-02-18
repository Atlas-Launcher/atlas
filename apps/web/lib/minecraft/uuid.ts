export function canonicalizeMinecraftUuid(value: string | null | undefined): string | null {
  const lower = (value ?? "").trim().toLowerCase();
  const candidate = lower.startsWith("urn:uuid:")
    ? lower.slice("urn:uuid:".length)
    : lower;
  const hex = candidate.replace(/[^0-9a-f]/g, "");
  return hex.length === 32 ? hex : null;
}
