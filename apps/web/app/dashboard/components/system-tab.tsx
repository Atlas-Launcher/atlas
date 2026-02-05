"use client";

import { useEffect, useMemo, useState } from "react";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import UserCard from "@/app/dashboard/components/user-card";
import type { UserSummary } from "@/app/dashboard/types";

export default function SystemTab() {
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
            />
          ))}
        </div>
      ) : (
        <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-8 text-sm text-[var(--atlas-ink-muted)]">
          No users found.
        </div>
      )}

    </div>
  );
}
