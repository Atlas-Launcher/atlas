"use client";

import Link from "next/link";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Role, UserSummary } from "@/app/dashboard/types";

interface UserCardProps {
  user: UserSummary;
  role: Role;
  onRoleChange: (role: Role) => void;
  onSaveRole: () => void;
  saving: boolean;
  isSelf: boolean;
}

const roleLabels: Record<Role, string> = {
  player: "Player",
  creator: "Creator",
  admin: "Admin",
};

export default function UserCard({
  user,
  role,
  onRoleChange,
  onSaveRole,
  saving,
  isSelf,
}: UserCardProps) {
  return (
    <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold">{user.name}</p>
          <p className="text-xs text-[var(--atlas-ink-muted)]">{user.email}</p>
        </div>
        <Badge variant="outline" className="text-[10px] uppercase tracking-[0.2em]">
          {roleLabels[user.role]}
        </Badge>
      </div>

      <div className="mt-4 space-y-3">
        <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
          Role
          <select
            value={role}
            onChange={(event) => onRoleChange(event.target.value as Role)}
            className="mt-2 h-11 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-3 text-sm"
            disabled={saving || isSelf}
          >
            <option value="player">Player</option>
            <option value="creator">Creator</option>
            <option value="admin">Admin</option>
          </select>
        </label>
        <div className="flex items-center justify-between gap-3">
          <Button
            size="sm"
            variant="outline"
            onClick={onSaveRole}
            disabled={saving || isSelf}
          >
            {saving ? "Saving..." : isSelf ? "Self managed" : "Update role"}
          </Button>
          <Link href={`/dashboard/admin/user/${user.id}`}>
            <Button size="sm">Manage user</Button>
          </Link>
        </div>
      </div>
    </div>
  );
}
