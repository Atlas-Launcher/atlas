"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import PackAccessDialog from "@/app/dashboard/components/pack-access-dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { ApiKey, Invite, PackMember } from "@/app/dashboard/types";

interface AccessTabProps {
  canManageInvites: boolean;
  invites: Invite[];
  inviteEmail: string;
  inviteRole: string;
  inviteAccess: string;
  onInviteEmailChange: (value: string) => void;
  onInviteRoleChange: (value: string) => void;
  onInviteAccessChange: (value: string) => void;
  onCreateInvite: () => void;
  members: PackMember[];
  onRevokeMember: (userId: string) => void;
  canManageApiKeys: boolean;
  apiKeyRecords: ApiKey[];
  apiKeyLabel: string;
  newApiKey: string | null;
  onApiKeyLabelChange: (value: string) => void;
  onCreateApiKey: () => void;
  loading: boolean;
  selectedPackId: string;
  currentUserId: string;
}

export default function AccessTab({
  canManageInvites,
  invites,
  inviteEmail,
  inviteRole,
  inviteAccess,
  onInviteEmailChange,
  onInviteRoleChange,
  onInviteAccessChange,
  onCreateInvite,
  members,
  onRevokeMember,
  canManageApiKeys,
  apiKeyRecords,
  apiKeyLabel,
  newApiKey,
  onApiKeyLabelChange,
  onCreateApiKey,
  loading,
  selectedPackId,
  currentUserId,
}: AccessTabProps) {
  const inviteLink = `/invite?pack=${selectedPackId}`;

  return (
    <div className="grid gap-6 md:grid-cols-2">
      {canManageInvites ? (
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Invite Ledger</CardTitle>
            <CardDescription>Share access links with players or creators.</CardDescription>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Link</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Access</TableHead>
                  <TableHead>Status</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {invites.map((invite) => (
                  <TableRow key={invite.id}>
                    <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                      {invite.packId ? `/invite?pack=${invite.packId}` : "—"}
                    </TableCell>
                    <TableCell>{invite.role}</TableCell>
                    <TableCell>{invite.accessLevel}</TableCell>
                    <TableCell>
                      {invite.usedAt ? (
                        <Badge variant="outline">Used</Badge>
                      ) : (
                        <Badge variant="secondary">Active</Badge>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      ) : null}

      {canManageInvites ? (
        <Card>
          <CardHeader>
            <CardTitle>Access Controls</CardTitle>
            <CardDescription>Invite teammates or revoke access.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3 text-xs text-[var(--atlas-ink-muted)]">
              Share this link: <span className="font-semibold text-[var(--atlas-ink)]">{inviteLink}</span>
            </div>
            <PackAccessDialog
              canManage={canManageInvites}
              inviteEmail={inviteEmail}
              inviteRole={inviteRole}
              inviteAccess={inviteAccess}
              onInviteEmailChange={onInviteEmailChange}
              onInviteRoleChange={onInviteRoleChange}
              onInviteAccessChange={onInviteAccessChange}
              onCreateInvite={onCreateInvite}
              members={members}
              onRevokeMember={onRevokeMember}
              loading={loading}
              currentUserId={currentUserId}
            />
          </CardContent>
        </Card>
      ) : null}

      {canManageApiKeys ? (
        <Card>
          <CardHeader>
            <CardTitle>Deploy API Keys</CardTitle>
            <CardDescription>Used by CI to upload new builds.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {newApiKey ? (
              <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
                Copy this key now: {newApiKey}
              </div>
            ) : null}
            <Input
              placeholder="Label (optional)"
              value={apiKeyLabel}
              onChange={(event) => onApiKeyLabelChange(event.target.value)}
            />
            <Button onClick={onCreateApiKey} disabled={loading || !selectedPackId}>
              Generate Key
            </Button>
            <div className="space-y-2 text-xs text-[var(--atlas-ink-muted)]">
              {apiKeyRecords.length ? (
                apiKeyRecords.map((key) => (
                  <div
                    key={key.id}
                    className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/60 px-3 py-2"
                  >
                    <span>{key.name || "Deploy key"}</span>
                    <Badge variant={key.enabled ? "secondary" : "outline"}>
                      {key.enabled ? "Active" : "Disabled"}
                    </Badge>
                  </div>
                ))
              ) : (
                <span>No deploy keys yet.</span>
              )}
            </div>
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}
