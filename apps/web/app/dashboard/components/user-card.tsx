"use client";

import Link from "next/link";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Role, UserSummary } from "@/app/dashboard/types";

interface UserCardProps {
  user: UserSummary;
}

const roleLabels: Record<Role, string> = {
  player: "Player",
  creator: "Creator",
  admin: "Admin",
};

export default function UserCard({ user }: UserCardProps) {
  return (
    <div className="atlas-panel rounded-2xl p-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-semibold">{user.name}</p>
          <p className="text-xs text-[var(--atlas-ink-muted)]">{user.email}</p>
        </div>
        <Badge variant="outline" className="text-[10px] uppercase tracking-[0.2em]">
          {roleLabels[user.role]}
        </Badge>
      </div>

      <div className="mt-4 flex items-center justify-end">
        <Link href={`/dashboard/admin/user/${user.id}`}>
          <Button size="sm">Manage user</Button>
        </Link>
      </div>
    </div>
  );
}
