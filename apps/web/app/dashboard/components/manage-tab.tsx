"use client";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { RunnerServiceToken } from "@/app/dashboard/types";

interface ManageTabProps {
  packName: string;
  canManageFriendlyName: boolean;
  friendlyName: string;
  onFriendlyNameChange: (value: string) => void;
  savingFriendlyName: boolean;
  onSaveFriendlyName: () => void;
  canDeletePack: boolean;
  deleteConfirmation: string;
  onDeleteConfirmationChange: (value: string) => void;
  deletingPack: boolean;
  onDeletePack: () => void;
  runnerTokens: RunnerServiceToken[];
  canManageRunnerTokens: boolean;
  runnerTokenName: string;
  onRunnerTokenNameChange: (value: string) => void;
  onCreateRunnerToken: () => void;
  createdRunnerToken: string | null;
  onRevokeRunnerToken: (tokenId: string) => void;
  loading: boolean;
}

export default function ManageTab({
  packName,
  canManageFriendlyName,
  friendlyName,
  onFriendlyNameChange,
  savingFriendlyName,
  onSaveFriendlyName,
  canDeletePack,
  deleteConfirmation,
  onDeleteConfirmationChange,
  deletingPack,
  onDeletePack,
  runnerTokens,
  canManageRunnerTokens,
  runnerTokenName,
  onRunnerTokenNameChange,
  onCreateRunnerToken,
  createdRunnerToken,
  onRevokeRunnerToken,
  loading,
}: ManageTabProps) {
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

  return (
    <Card>
      <CardHeader>
        <CardTitle>Manage pack</CardTitle>
        <CardDescription>Update pack settings and service tokens.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="inline-block w-[42rem] max-w-full rounded-2xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] p-4">
          <h3 className="text-sm font-semibold">Pack name</h3>
          {canManageFriendlyName ? (
            <div className="mt-3 flex flex-wrap items-center gap-3">
              <Input
                className="max-w-md"
                value={friendlyName}
                onChange={(event) => onFriendlyNameChange(event.target.value)}
                placeholder={packName}
              />
              <Button onClick={onSaveFriendlyName} disabled={savingFriendlyName}>
                {savingFriendlyName ? "Saving..." : "Save name"}
              </Button>
            </div>
          ) : (
            <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{packName}</p>
          )}
        </div>

        {canDeletePack ? (
          <div className="inline-block w-[42rem] max-w-full space-y-4 rounded-2xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] p-4">
            <h3 className="text-lg font-semibold">Delete pack</h3>
            <div className="inline-block w-[42rem] max-w-full rounded-2xl border border-amber-300 bg-amber-50 p-4">
              <p className="text-sm text-amber-800">
                Deleting a pack removes build history, channels, invites, and access records from
                Atlas.
              </p>
              <p className="text-sm text-destructive font-bold">This action does not delete the GitHub repository.</p>
            </div>
            <label className="mt-3 block text-xs font-semibold uppercase tracking-[0.1em] text-[var(--atlas-ink-muted)]">
              Type the pack name to confirm
              <Input
                className="mt-2"
                placeholder={packName}
                value={deleteConfirmation}
                onChange={(event) => onDeleteConfirmationChange(event.target.value)}
              />
            </label>
            <Button className="mt-3" variant="destructive" onClick={onDeletePack} disabled={deletingPack}>
              {deletingPack ? "Deleting..." : "Delete pack"}
            </Button>
          </div>
        ) : (
          <p className="text-sm text-[var(--atlas-ink-muted)]">
            You do not have permission to delete this pack.
          </p>
        )}

        <div className="inline-block w-[48rem] max-w-full rounded-2xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <h3 className="text-sm font-semibold">Runner Service Tokens</h3>
              <p className="text-xs text-[var(--atlas-ink-muted)]">
                Manage deploy keys used by Atlas Runner.
              </p>
            </div>
          </div>
          {canManageRunnerTokens ? (
            <div className="mt-3 flex flex-wrap items-center gap-3">
              <Input
                placeholder="Token name (optional)"
                value={runnerTokenName}
                onChange={(event) => onRunnerTokenNameChange(event.target.value)}
                className="max-w-sm"
              />
              <Button size="sm" onClick={onCreateRunnerToken} disabled={loading}>
                {loading ? "Creating..." : "Create runner token"}
              </Button>
            </div>
          ) : null}

          {createdRunnerToken ? (
            <div className="mt-3 rounded-xl border border-amber-300 bg-amber-50 px-3 py-2">
              <p className="text-xs font-semibold text-amber-900">New token (shown once):</p>
              <p className="mt-1 break-all font-mono text-xs text-amber-900">
                {createdRunnerToken}
              </p>
            </div>
          ) : null}
          <div className="mt-4">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Name</TableHead>
                  <TableHead>Prefix</TableHead>
                  <TableHead>Created</TableHead>
                  <TableHead>Last Used</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="w-[140px] text-right">Action</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {runnerTokens.length ? (
                  runnerTokens.map((token) => (
                    <TableRow key={token.id}>
                      <TableCell className="font-medium">
                        {token.name ?? "Unnamed token"}
                      </TableCell>
                      <TableCell className="font-mono text-xs">
                        {token.tokenPrefix}
                      </TableCell>
                      <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                        {formatDate(token.createdAt)}
                      </TableCell>
                      <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                        {formatDate(token.lastUsedAt)}
                      </TableCell>
                      <TableCell className="text-xs">
                        {token.revokedAt ? "Revoked" : "Active"}
                      </TableCell>
                      <TableCell className="text-right">
                        {canManageRunnerTokens ? (
                          <Button
                            size="sm"
                            variant="outline"
                            disabled={loading || Boolean(token.revokedAt)}
                            onClick={() => onRevokeRunnerToken(token.id)}
                          >
                            {token.revokedAt ? "Revoked" : "Revoke"}
                          </Button>
                        ) : (
                          <span className="text-xs text-[var(--atlas-ink-muted)]">—</span>
                        )}
                      </TableCell>
                    </TableRow>
                  ))
                ) : (
                  <TableRow>
                    <TableCell colSpan={6} className="text-sm text-[var(--atlas-ink-muted)]">
                      No runner service tokens found for this pack.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
