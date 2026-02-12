"use client";

import { useEffect, useMemo, useState } from "react";

import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import UserCard from "@/app/dashboard/components/user-card";
import type { AppDeployToken, UserSummary } from "@/app/dashboard/types";

export default function SystemTab() {
  const [users, setUsers] = useState<UserSummary[]>([]);
  const [appDeployTokens, setAppDeployTokens] = useState<AppDeployToken[]>([]);
  const [query, setQuery] = useState("");
  const [tokenName, setTokenName] = useState("");
  const [createdToken, setCreatedToken] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [tokenLoading, setTokenLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tokenError, setTokenError] = useState<string | null>(null);

  const filteredUsers = useMemo(() => {
    const normalized = query.toLowerCase();
    return users.filter((user) => {
      const haystack = `${user.name} ${user.email} ${user.role}`.toLowerCase();
      return haystack.includes(normalized);
    });
  }, [users, query]);

  const formatDate = (value?: string | null) => {
    if (!value) {
      return "—";
    }
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return "—";
    }
    return date.toLocaleString();
  };

  const loadAppDeployTokens = async () => {
    setTokenLoading(true);
    setTokenError(null);
    const response = await fetch("/api/v1/deploy/app-tokens");
    const data = await response.json();
    setTokenLoading(false);

    if (!response.ok) {
      setTokenError(data?.error ?? "Unable to load app deploy keys.");
      return;
    }

    setAppDeployTokens(data.tokens ?? []);
  };

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

  const createAppDeployToken = async () => {
    setTokenLoading(true);
    setTokenError(null);
    setCreatedToken(null);

    const response = await fetch("/api/v1/deploy/app-tokens", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        name: tokenName.trim() || null,
      }),
    });
    const data = await response.json();
    setTokenLoading(false);

    if (!response.ok) {
      setTokenError(data?.error ?? "Unable to create app deploy key.");
      return;
    }

    setCreatedToken(data?.token ?? null);
    setTokenName("");
    await loadAppDeployTokens();
  };

  const revokeAppDeployToken = async (tokenId: string) => {
    setTokenLoading(true);
    setTokenError(null);
    const response = await fetch("/api/v1/deploy/app-tokens", {
      method: "DELETE",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ tokenId }),
    });
    const data = await response.json().catch(() => ({}));
    setTokenLoading(false);
    if (!response.ok) {
      setTokenError(data?.error ?? "Unable to revoke app deploy key.");
      return;
    }

    await loadAppDeployTokens();
  };

  const refreshAll = async () => {
    await Promise.all([loadUsers(), loadAppDeployTokens()]);
  };

  useEffect(() => {
    refreshAll().catch(() => {
      setError("Unable to load dashboard system data.");
      setTokenError("Unable to load app deploy keys.");
    });
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
          {loading || tokenLoading ? "Refreshing..." : "Refresh"}
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

      <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-white/70 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-lg font-semibold">App Deploy Keys</h2>
            <p className="text-xs text-[var(--atlas-ink-muted)]">
              Manage deploy keys used by app release workflows.
            </p>
          </div>
        </div>

        <div className="mt-3 flex flex-wrap items-center gap-3">
          <Input
            placeholder="Key name (optional)"
            value={tokenName}
            onChange={(event) => setTokenName(event.target.value)}
            className="max-w-sm"
          />
          <Button size="sm" onClick={createAppDeployToken} disabled={tokenLoading}>
            {tokenLoading ? "Creating..." : "Create app deploy key"}
          </Button>
        </div>

        {createdToken ? (
          <div className="mt-3 rounded-xl border border-amber-300 bg-amber-50 px-3 py-2">
            <p className="text-xs font-semibold text-amber-900">
              New key (shown once):
            </p>
            <p className="mt-1 break-all font-mono text-xs text-amber-900">
              {createdToken}
            </p>
          </div>
        ) : null}

        {tokenError ? (
          <div className="mt-3 rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
            {tokenError}
          </div>
        ) : null}

        <div className="mt-4 overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Prefix</TableHead>
                <TableHead>Created</TableHead>
                <TableHead>Last Used</TableHead>
                <TableHead>Expires</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="text-right">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {appDeployTokens.length ? (
                appDeployTokens.map((token) => (
                  <TableRow key={token.id}>
                    <TableCell className="font-medium">{token.name ?? "Unnamed key"}</TableCell>
                    <TableCell className="font-mono text-xs">{token.tokenPrefix}</TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {formatDate(token.createdAt)}
                    </TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {formatDate(token.lastUsedAt)}
                    </TableCell>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {formatDate(token.expiresAt)}
                    </TableCell>
                    <TableCell className="text-xs">
                      {token.revokedAt ? "Revoked" : "Active"}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={tokenLoading || Boolean(token.revokedAt)}
                        onClick={() => revokeAppDeployToken(token.id)}
                      >
                        {token.revokedAt ? "Revoked" : "Revoke"}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              ) : (
                <TableRow>
                  <TableCell colSpan={7} className="text-sm text-[var(--atlas-ink-muted)]">
                    No app deploy keys created yet.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </div>
      </div>

    </div>
  );
}
