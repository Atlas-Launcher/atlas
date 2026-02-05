"use client";

import { useMemo } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import type { PackMember } from "@/app/dashboard/types";

interface PackAccessDialogProps {
  canManage: boolean;
  inviteEmail: string;
  inviteRole: string;
  inviteAccess: string;
  onInviteEmailChange: (value: string) => void;
  onInviteRoleChange: (value: string) => void;
  onInviteAccessChange: (value: string) => void;
  onCreateInvite: () => void;
  members: PackMember[];
  onRevokeMember: (userId: string) => void;
  loading: boolean;
  currentUserId: string;
}

export default function PackAccessDialog({
  canManage,
  inviteEmail,
  inviteRole,
  inviteAccess,
  onInviteEmailChange,
  onInviteRoleChange,
  onInviteAccessChange,
  onCreateInvite,
  members,
  onRevokeMember,
  loading,
  currentUserId,
}: PackAccessDialogProps) {
  const sortedMembers = useMemo(
    () =>
      [...members].sort((a, b) => a.name.localeCompare(b.name)),
    [members]
  );

  if (!canManage) {
    return null;
  }

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button variant="outline">Manage Access</Button>
      </DialogTrigger>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Pack Access</DialogTitle>
          <DialogDescription>
            Invite new teammates or revoke existing access instantly.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold">Send Invite</h3>
              <p className="text-xs text-[var(--atlas-ink-muted)]">
                Create a new invite link for a player or creator.
              </p>
            </div>
            <Input
              placeholder="Email (optional)"
              value={inviteEmail}
              onChange={(event) => onInviteEmailChange(event.target.value)}
            />
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Role
                <select
                  value={inviteRole}
                  onChange={(event) => onInviteRoleChange(event.target.value)}
                  className="mt-2 h-11 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-3 text-sm"
                >
                  <option value="player">Player</option>
                  <option value="creator">Creator</option>
                </select>
              </label>
              <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Access Level
                <select
                  value={inviteAccess}
                  onChange={(event) => onInviteAccessChange(event.target.value)}
                  className="mt-2 h-11 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-3 text-sm"
                >
                  <option value="production">Production</option>
                  <option value="beta">Beta</option>
                  <option value="dev">Dev</option>
                </select>
              </label>
            </div>
            <Button onClick={onCreateInvite} disabled={loading}>
              {loading ? "Creating..." : "Send invite"}
            </Button>
          </div>

          <Separator />

          <div className="space-y-3">
            <div>
              <h3 className="text-sm font-semibold">Members</h3>
              <p className="text-xs text-[var(--atlas-ink-muted)]">
                Revoke access for anyone who should no longer be in the pack.
              </p>
            </div>
            {sortedMembers.length ? (
              <div className="space-y-2">
                {sortedMembers.map((member) => (
                  <div
                    key={member.userId}
                    className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/60 px-3 py-2"
                  >
                    <div>
                      <p className="text-sm font-semibold">{member.name}</p>
                      <p className="text-xs text-[var(--atlas-ink-muted)]">{member.email}</p>
                    </div>
                    <div className="flex items-center gap-2 text-xs">
                      <Badge variant="secondary">{member.role}</Badge>
                      <Badge variant="outline">{member.accessLevel}</Badge>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => onRevokeMember(member.userId)}
                        disabled={loading || member.userId === currentUserId}
                      >
                        {member.userId === currentUserId ? "You" : "Revoke"}
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-xs text-[var(--atlas-ink-muted)]">No members found.</p>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
