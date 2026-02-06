"use client";

import { useMemo, useState } from "react";

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
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { AccessLevel, ApiKey, Invite, PackMember } from "@/app/dashboard/types";

function accessLabel(level: AccessLevel) {
  if (level === "all") {
    return "Dev + Beta + Prod";
  }
  if (level === "dev") {
    return "Dev + Prod";
  }
  if (level === "beta") {
    return "Beta + Prod";
  }
  return "Prod only";
}

interface AccessTabProps {
  canManageInvites: boolean;
  invites: Invite[];
  onCreateInvite: () => void;
  onDeleteInvite: (inviteId: string) => void;
  inviteLinkModal: string | null;
  onCloseInviteLinkModal: () => void;
  members: PackMember[];
  onRevokeMember: (userId: string) => void;
  canManageApiKeys: boolean;
  apiKeyRecords: ApiKey[];
  apiKeyLabel: string;
  newApiKey: string | null;
  onApiKeyLabelChange: (value: string) => void;
  onCreateApiKey: () => void;
  loading: boolean;
  currentUserId: string;
  canManageMembers: boolean;
}

export default function AccessTab({
  canManageInvites,
  invites,
  onCreateInvite,
  onDeleteInvite,
  inviteLinkModal,
  onCloseInviteLinkModal,
  members,
  onRevokeMember,
  canManageApiKeys,
  apiKeyRecords,
  apiKeyLabel,
  newApiKey,
  onApiKeyLabelChange,
  onCreateApiKey,
  loading,
  currentUserId,
  canManageMembers,
}: AccessTabProps) {
  const sortedMembers = useMemo(
    () => [...members].sort((a, b) => a.name.localeCompare(b.name)),
    [members]
  );
  const [copied, setCopied] = useState(false);
  const activeInvites = useMemo(
    () => invites.filter((invite) => !invite.usedAt),
    [invites]
  );

  const formatCreatedAt = (value?: string | null) => {
    if (!value) {
      return "—";
    }
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) {
      return "—";
    }
    return date.toLocaleString();
  };

  const handleCopyInviteLink = async () => {
    if (!inviteLinkModal) {
      return;
    }

    await navigator.clipboard.writeText(inviteLinkModal);
    setCopied(true);
  };

  const handleModalOpenChange = (open: boolean) => {
    if (!open) {
      setCopied(false);
      onCloseInviteLinkModal();
    }
  };
  return (
    <>
      <Dialog open={Boolean(inviteLinkModal)} onOpenChange={handleModalOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Invite Link Ready</DialogTitle>
            <DialogDescription>Copy and share this full invite URL.</DialogDescription>
          </DialogHeader>
          <Input readOnly value={inviteLinkModal ?? ""} />
          <DialogFooter className="pt-2">
            <Button onClick={handleCopyInviteLink}>{copied ? "Copied" : "Copy Link"}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <div className="grid gap-6 md:grid-cols-2">
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Access</CardTitle>
            <CardDescription>Manage collaborators and pending invites.</CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs defaultValue="users">
              <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
                <TabsList>
                  <TabsTrigger value="users">Users</TabsTrigger>
                  <TabsTrigger value="invites">Active Invites</TabsTrigger>
                </TabsList>
                {canManageInvites ? (
                  <Button onClick={onCreateInvite} disabled={loading}>
                    {loading ? "Creating..." : "Create Invite"}
                  </Button>
                ) : null}
              </div>

              <TabsContent value="users">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>User</TableHead>
                      <TableHead>Email</TableHead>
                      <TableHead>Role</TableHead>
                      <TableHead>Access</TableHead>
                      <TableHead className="w-[120px] text-right">Action</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {sortedMembers.length ? (
                      sortedMembers.map((member) => (
                        <TableRow key={member.userId}>
                          <TableCell className="font-medium">{member.name}</TableCell>
                          <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                            {member.email}
                          </TableCell>
                          <TableCell>
                            <Badge variant="secondary">{member.role}</Badge>
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline">{accessLabel(member.accessLevel)}</Badge>
                          </TableCell>
                          <TableCell className="text-right">
                            {canManageMembers && member.userId !== currentUserId ? (
                              <Button
                                size="sm"
                                variant="outline"
                                onClick={() => onRevokeMember(member.userId)}
                                disabled={loading}
                              >
                                Remove
                              </Button>
                            ) : (
                              <span className="text-xs text-[var(--atlas-ink-muted)]">
                                {member.userId === currentUserId ? "You" : "—"}
                              </span>
                            )}
                          </TableCell>
                        </TableRow>
                      ))
                    ) : (
                      <TableRow>
                        <TableCell colSpan={5} className="text-sm text-[var(--atlas-ink-muted)]">
                          No users found for this pack.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </TabsContent>

              <TabsContent value="invites">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Link</TableHead>
                      <TableHead>Created At</TableHead>
                      <TableHead className="w-[120px] text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {activeInvites.length ? (
                      activeInvites.map((invite) => (
                        <TableRow key={invite.id}>
                          <TableCell className="max-w-[680px] text-xs text-[var(--atlas-ink-muted)]">
                            <span className="break-all">
                              {invite.inviteUrl ?? `/invite?code=${invite.code}`}
                            </span>
                          </TableCell>
                          <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                            {formatCreatedAt(invite.createdAt)}
                          </TableCell>
                          <TableCell className="text-right">
                            {canManageInvites ? (
                              <Button
                                size="sm"
                                variant="destructive"
                                onClick={() => onDeleteInvite(invite.id)}
                                disabled={loading}
                              >
                                Delete
                              </Button>
                            ) : (
                              <span className="text-xs text-[var(--atlas-ink-muted)]">—</span>
                            )}
                          </TableCell>
                        </TableRow>
                      ))
                    ) : (
                      <TableRow>
                        <TableCell colSpan={3} className="text-sm text-[var(--atlas-ink-muted)]">
                          No active invites.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

        {canManageApiKeys ? (
          <Card className="md:col-span-2">
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
              <Button onClick={onCreateApiKey} disabled={loading}>
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
    </>
  );
}
