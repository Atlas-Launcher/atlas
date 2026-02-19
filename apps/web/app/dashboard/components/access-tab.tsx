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
import type { AccessLevel, Invite, PackMember } from "@/app/dashboard/types";

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
  onPromoteMember: (userId: string) => void;
  onDemoteMember: (userId: string) => void;
  onUpdateAccessLevel: (userId: string, accessLevel: AccessLevel) => void;
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
  onPromoteMember,
  onDemoteMember,
  onUpdateAccessLevel,
  loading,
  currentUserId,
  canManageMembers,
}: AccessTabProps) {
  const sortedMembers = useMemo(
    () => [...members].sort((a, b) => a.name.localeCompare(b.name)),
    [members]
  );
  const [copied, setCopied] = useState(false);
  const activeInvites = useMemo(() => invites, [invites]);

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
            <DialogTitle>Invite link ready</DialogTitle>
            <DialogDescription>Copy and share this invite URL.</DialogDescription>
          </DialogHeader>
          <Input readOnly value={inviteLinkModal ?? ""} />
          <DialogFooter className="pt-2">
            <Button onClick={handleCopyInviteLink}>{copied ? "Copied" : "Copy link"}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <div className="grid gap-6 md:grid-cols-2">
        <Card className="md:col-span-2">
          <CardHeader>
            <CardTitle>Access</CardTitle>
            <CardDescription>
              Manage collaborators and active invite links. If a promoted creator has no linked
              GitHub account, you must add them to the repository manually.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs defaultValue="users">
              <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
                <TabsList>
                  <TabsTrigger value="users">Users</TabsTrigger>
                  <TabsTrigger value="invites">Invites</TabsTrigger>
                </TabsList>
                {canManageInvites ? (
                  <Button onClick={onCreateInvite} disabled={loading}>
                    {loading ? "Creating..." : "Create invite"}
                  </Button>
                ) : null}
              </div>

              <TabsContent value="users">
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>User</TableHead>
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
                          <TableCell>
                            <Badge variant="secondary">{member.role}</Badge>
                          </TableCell>
                          <TableCell>
                            {canManageMembers ? (
                              <select
                                className="h-8 rounded-md border border-input bg-background px-2 text-xs"
                                value={member.accessLevel}
                                onChange={(event) =>
                                  onUpdateAccessLevel(
                                    member.userId,
                                    event.target.value as AccessLevel
                                  )
                                }
                                disabled={loading}
                              >
                                <option value="production">Prod only</option>
                                <option value="beta">Beta + Prod</option>
                                <option value="dev">Dev + Prod</option>
                                <option value="all">Dev + Beta + Prod</option>
                              </select>
                            ) : (
                              <Badge variant="outline">{accessLabel(member.accessLevel)}</Badge>
                            )}
                          </TableCell>
                          <TableCell className="text-right">
                            {canManageMembers && member.userId !== currentUserId ? (
                              <div className="flex justify-end gap-2">
                                {member.role === "player" ? (
                                  <Button
                                    size="sm"
                                    onClick={() => onPromoteMember(member.userId)}
                                    disabled={loading}
                                  >
                                    Make creator
                                  </Button>
                                ) : null}
                                {member.role === "creator" ? (
                                  <Button
                                    size="sm"
                                    variant="secondary"
                                    onClick={() => onDemoteMember(member.userId)}
                                    disabled={loading}
                                  >
                                    Make player
                                  </Button>
                                ) : null}
                                <Button
                                  size="sm"
                                  variant="outline"
                                  onClick={() => onRevokeMember(member.userId)}
                                  disabled={loading}
                                >
                                  Remove
                                </Button>
                              </div>
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
                        <TableCell colSpan={4} className="text-sm text-[var(--atlas-ink-muted)]">
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
                                Revoke
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
                          No invites yet.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>

      </div>
    </>
  );
}
