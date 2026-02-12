export type Role = "admin" | "creator" | "player";
export type AccessLevel = "dev" | "beta" | "production" | "all";

export interface Pack {
  id: string;
  name: string;
  slug: string;
  description?: string | null;
  repoUrl?: string | null;
  createdAt?: string;
  updatedAt?: string;
}

export interface Build {
  id: string;
  version: string;
  commitHash?: string | null;
  commitMessage?: string | null;
  minecraftVersion?: string | null;
  modloader?: string | null;
  modloaderVersion?: string | null;
  forceReinstall?: boolean;
  artifactKey: string;
  artifactProvider?: "r2" | "vercel_blob" | null;
  createdAt?: string;
}

export interface Channel {
  id: string;
  name: "dev" | "beta" | "production";
  buildId?: string | null;
  buildVersion?: string | null;
  buildCommit?: string | null;
  updatedAt?: string;
}

export interface Invite {
  id: string;
  email?: string | null;
  code: string;
  inviteUrl?: string;
  packId?: string | null;
  role: Role;
  accessLevel: AccessLevel;
  expiresAt?: string | null;
  usedAt?: string | null;
  createdAt?: string;
}

export interface UserSummary {
  id: string;
  name: string;
  email: string;
  role: Role;
  createdAt?: string;
}

export interface PackMember {
  userId: string;
  name: string;
  email: string;
  role: Role;
  accessLevel: AccessLevel;
  joinedAt?: string;
}

export interface UserMembership {
  packId: string;
  packName: string;
  packSlug: string;
  role: Role;
  accessLevel: AccessLevel;
  joinedAt?: string;
}

export interface RunnerServiceToken {
  id: string;
  name?: string | null;
  tokenPrefix: string;
  createdAt?: string | null;
  lastUsedAt?: string | null;
  revokedAt?: string | null;
  expiresAt?: string | null;
}

export interface AppDeployToken {
  id: string;
  name?: string | null;
  tokenPrefix: string;
  createdAt?: string | null;
  lastUsedAt?: string | null;
  revokedAt?: string | null;
  expiresAt?: string | null;
}
