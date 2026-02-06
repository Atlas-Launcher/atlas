export const STORAGE_PROVIDERS = ["r2", "vercel_blob"] as const;

export type StorageProviderId = (typeof STORAGE_PROVIDERS)[number];

export interface ArtifactRef {
  provider: StorageProviderId;
  key: string;
}
