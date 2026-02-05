export type Role = "admin" | "creator" | "player";

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
  artifactKey: string;
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
  packId?: string | null;
  role: Role;
  accessLevel: "dev" | "beta" | "production";
  usedAt?: string | null;
  createdAt?: string;
}

export interface ApiKey {
  id: string;
  name?: string | null;
  start?: string | null;
  enabled: boolean;
  createdAt?: string;
  metadata?: {
    packId?: string | null;
    type?: string | null;
  } | null;
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
  accessLevel: "dev" | "beta" | "production";
  joinedAt?: string;
}

export interface UserMembership {
  packId: string;
  packName: string;
  packSlug: string;
  role: Role;
  accessLevel: "dev" | "beta" | "production";
  joinedAt?: string;
}
