"use client";

import { useEffect, useMemo, useState } from "react";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import UserCard from "@/app/dashboard/components/user-card";
import type { Role, UserSummary } from "@/app/dashboard/types";

interface SystemTabProps {
  currentUserId: string;
}

export default function SystemTab({ currentUserId }: SystemTabProps) {
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editingRoles, setEditingRoles] = useState<Record<string, Role>>({});
  const [savingUserId, setSavingUserId] = useState<string | null>(null);

  const filteredUsers = useMemo(() => {
    const normalized = query.toLowerCase();
    return users.filter((user) => {
      const haystack = `${user.name} ${user.email} ${user.role}`.toLowerCase();
      return haystack.includes(normalized);
    });
  }, [users, query]);

  const loadUsers = async () => {
    setLoading(true);
    setError(null);
    const response = await fetch("/api/admin/users");
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to load users.");
      return;
    }

    setUsers(data.users ?? []);
    setEditingRoles(
      (data.users ?? []).reduce((acc: Record<string, Role>, user: UserSummary) => {
        acc[user.id] = user.role;
        return acc;
      }, {})
    );
  };

  const handleRoleChange = (userId: string, role: Role) => {
    setEditingRoles((prev) => ({ ...prev, [userId]: role }));
  };

  const handleSaveRole = async (userId: string) => {
    const role = editingRoles[userId];
    if (!role) {
      return;
    }

    setSavingUserId(userId);
    setError(null);
    const response = await fetch(`/api/admin/users/${userId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role }),
    });
    const data = await response.json();
    setSavingUserId(null);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update role.");
      return;
    }

    setUsers((prev) =>
      prev.map((user) => (user.id === userId ? { ...user, role: data.user.role } : user))
    );
  };

  useEffect(() => {
    loadUsers().catch(() => setError("Unable to load users."));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center gap-3">
        <Input
          placeholder="Search users..."
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          className="min-w-[220px] flex-1"
        />
        <Button size="sm" variant="outline" onClick={loadUsers} disabled={loading}>
          {loading ? "Refreshing..." : "Refresh"}
        </Button>
      </div>

      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">System Users</h2>
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            {filteredUsers.length} users
          </p>
        </div>
      </div>

      {error ? (
        <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
          {error}
        </div>
      ) : null}

      {filteredUsers.length ? (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {filteredUsers.map((user) => (
            <UserCard
              key={user.id}
              user={user}
              role={editingRoles[user.id] ?? user.role}
              onRoleChange={(role) => handleRoleChange(user.id, role)}
              onSaveRole={() => handleSaveRole(user.id)}
              saving={savingUserId === user.id}
              isSelf={user.id === currentUserId}
            />
          ))}
        </div>
      ) : (
        <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 text-sm text-[var(--atlas-ink-muted)]">
          No users found.
        </div>
      )}

      {currentUserId ? (
        <p className="text-xs text-[var(--atlas-ink-muted)]">
          Tip: Your own admin record can’t be demoted here.
        </p>
      ) : null}
    </div>
  );
}
