export const roleHierarchy = {
  player: 1,
  creator: 2,
  admin: 3,
} as const;

export type Role = keyof typeof roleHierarchy;

type SessionLike = { user?: { role?: Role | null } } | null;

export function hasRole(session: SessionLike, roles: Role[]) {
  if (!session?.user?.role) {
    return false;
  }

  return roles.includes(session.user.role as Role);
}

export function requireRole(session: SessionLike, roles: Role[]) {
  if (!hasRole(session, roles)) {
    throw new Error("FORBIDDEN");
  }
}

export function allowedChannels(
  accessLevel: "dev" | "beta" | "production" | "all",
  role?: Role | null
) {
  if (role === "admin") {
    return ["dev", "beta", "production"] as const;
  }

  if (accessLevel === "all") {
    return ["dev", "beta", "production"] as const;
  }

  if (accessLevel === "dev") {
    return ["dev", "production"] as const;
  }

  if (accessLevel === "beta") {
    return ["beta", "production"] as const;
  }

  return ["production"] as const;
}
