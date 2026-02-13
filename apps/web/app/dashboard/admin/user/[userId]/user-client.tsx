"use client";

import { useMemo, useState } from "react";
import Link from "next/link";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import SignOutButton from "@/app/dashboard/sign-out-button";
import type { Role, UserMembership, UserSummary } from "@/app/dashboard/types";

interface AdminUserClientProps {
  session: {
    user: {
      id: string;
      email: string;
      role: Role;
    };
  };
  user: UserSummary;
  memberships: UserMembership[];
}

const roleLabels: Record<Role, string> = {
  player: "Player",
  creator: "Creator",
  admin: "Admin",
};

export default function AdminUserClient({ session, user, memberships }: AdminUserClientProps) {
  const [role, setRole] = useState<Role>(user.role);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const isSelf = session.user.id === user.id;
  const memberCount = useMemo(() => memberships.length, [memberships.length]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    const response = await fetch(`/api/admin/users/${user.id}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role }),
    });
    const data = await response.json();
    setSaving(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update role.");
      return;
    }

    setRole(data.user.role);
  };

  return (
    <div className="min-h-screen bg-transparent px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <DashboardHeader
          workspaceName="System"
          email={session.user.email}
          role={session.user.role}
          actions={<SignOutButton />}
          tabs={
            <div className="flex items-center gap-2">
              <Link href="/dashboard?tab=system">
                <Button size="sm" variant="outline">
                  Back to System
                </Button>
              </Link>
              <Badge variant="secondary">User detail</Badge>
            </div>
          }
        />

        {error ? (
          <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
            {error}
          </div>
        ) : null}

        <div className="grid gap-6 lg:grid-cols-[1.1fr_1fr]">
          <Card>
            <CardHeader>
              <CardTitle>{user.name}</CardTitle>
              <CardDescription>{user.email}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex flex-wrap items-center gap-3">
                <Badge variant="outline" className="text-[10px] uppercase tracking-[0.2em]">
                  {roleLabels[user.role]}
                </Badge>
                <span className="text-xs text-[var(--atlas-ink-muted)]">
                  {memberCount} pack memberships
                </span>
              </div>

              <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Update Role
                <select
                  value={role}
                  onChange={(event) => setRole(event.target.value as Role)}
                  className="mt-2 h-11 w-full rounded-2xl border border-[hsl(var(--border)/0.95)] bg-white px-3 text-sm"
                  disabled={saving || isSelf}
                >
                  <option value="player">Player</option>
                  <option value="creator">Creator</option>
                  <option value="admin">Admin</option>
                </select>
              </label>
              <Button onClick={handleSave} disabled={saving || isSelf}>
                {saving ? "Saving..." : isSelf ? "Cannot change yourself" : "Save role"}
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Memberships</CardTitle>
              <CardDescription>Where this user currently has access.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {memberships.length ? (
                memberships.map((membership) => (
                  <div
                    key={membership.packId}
                    className="rounded-2xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-cream)]/60 px-4 py-3"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <div>
                        <p className="text-sm font-semibold">{membership.packName}</p>
                        <p className="text-xs text-[var(--atlas-ink-muted)]">
                          {membership.packSlug}
                        </p>
                      </div>
                      <Link href={`/dashboard/${membership.packId}`}>
                        <Button size="sm" variant="outline">
                          View pack
                        </Button>
                      </Link>
                    </div>
                    <div className="mt-3 flex flex-wrap gap-2 text-xs text-[var(--atlas-ink-muted)]">
                      <Badge variant="secondary">{membership.role}</Badge>
                      <Badge variant="outline">{membership.accessLevel}</Badge>
                    </div>
                  </div>
                ))
              ) : (
                <p className="text-sm text-[var(--atlas-ink-muted)]">
                  No pack access yet.
                </p>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
