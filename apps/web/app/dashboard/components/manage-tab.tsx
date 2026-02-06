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
}: ManageTabProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Manage Pack</CardTitle>
        <CardDescription>Update pack settings and Atlas-specific metadata.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="inline-block w-[42rem] max-w-full rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4">
          <h3 className="text-sm font-semibold">Pack Name</h3>
          {canManageFriendlyName ? (
            <div className="mt-3 flex flex-wrap items-center gap-3">
              <Input
                className="max-w-md"
                value={friendlyName}
                onChange={(event) => onFriendlyNameChange(event.target.value)}
                placeholder={packName}
              />
              <Button onClick={onSaveFriendlyName} disabled={savingFriendlyName}>
                {savingFriendlyName ? "Saving..." : "Change Name"}
              </Button>
            </div>
          ) : (
            <p className="mt-2 text-sm text-[var(--atlas-ink-muted)]">{packName}</p>
          )}
        </div>

        {canDeletePack ? (
          <div className="inline-block w-[42rem] max-w-full space-y-4 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 p-4">
            <h3 className="text-lg font-semibold">Delete Pack</h3>
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
      </CardContent>
    </Card>
  );
}
