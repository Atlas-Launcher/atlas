"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { ArrowLeft } from "lucide-react";

import BuildsTab from "@/app/dashboard/components/builds-tab";
import AccessTab from "@/app/dashboard/components/access-tab";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import ManageTab from "@/app/dashboard/components/manage-tab";
import SignOutButton from "@/app/dashboard/sign-out-button";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type {
  AccessLevel,
  Build,
  Channel,
  Invite,
  Pack,
  PackDeployToken,
  PackMember,
  Role,
  RunnerServiceToken,
} from "@/app/dashboard/types";

interface PackDashboardClientProps {
  session: {
    user: {
      id: string;
      email: string;
      role: Role;
    };
  };
  packId: string;
}

export default function PackDashboardClient({ session, packId }: PackDashboardClientProps) {
  const router = useRouter();
  const [pack, setPack] = useState<Pack | null>(null);
  const [builds, setBuilds] = useState<Build[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [invites, setInvites] = useState<Invite[]>([]);
  const [members, setMembers] = useState<PackMember[]>([]);
  const [runnerTokens, setRunnerTokens] = useState<RunnerServiceToken[]>([]);
  const [packDeployTokens, setPackDeployTokens] = useState<PackDeployToken[]>([]);
  const [runnerTokenName, setRunnerTokenName] = useState("");
  const [createdRunnerToken, setCreatedRunnerToken] = useState<string | null>(null);
  const [packDeployTokenName, setPackDeployTokenName] = useState("");
  const [createdPackDeployToken, setCreatedPackDeployToken] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [inviteLinkModal, setInviteLinkModal] = useState<string | null>(null);
  const [friendlyName, setFriendlyName] = useState("");
  const [savingFriendlyName, setSavingFriendlyName] = useState(false);
  const [deleteConfirmation, setDeleteConfirmation] = useState("");
  const [deletingPack, setDeletingPack] = useState(false);

  const canManage = session.user.role === "admin" || session.user.role === "creator";
  const canPromoteBuilds = canManage;
  const canManageInvites = canManage;
  const canManageMembers = canManage;
  const canManageFriendlyName = canManage;
  const canDeletePack = canManage;
  const canManageRunnerTokens = canManage;

  const packLabel = useMemo(() => pack?.slug ?? pack?.name ?? packId, [pack, packId]);
  const packName = pack?.name ?? packId;

  useEffect(() => {
    const loadPack = async () => {
      const response = await fetch("/api/v1/packs");
      const data = await response.json();
      if (!response.ok) {
        setError(data?.error ?? "Unable to load pack.");
        return;
      }

      const found = (data.packs ?? []).find((item: Pack) => item.id === packId) ?? null;
      setPack(found);
      setFriendlyName(found?.name ?? "");
      if (!found) {
        setError("You do not have access to this pack.");
      }
    };

    loadPack().catch(() => setError("Unable to load pack."));
  }, [packId]);

  useEffect(() => {
    const loadDetails = async () => {
      const [buildRes, channelRes, memberRes, inviteRes, packDeployTokenRes] = await Promise.all([
        fetch(`/api/v1/packs/${packId}/builds`),
        fetch(`/api/v1/packs/${packId}/channels`),
        fetch(`/api/v1/packs/${packId}/members`),
        canManageInvites ? fetch(`/api/v1/packs/${packId}/invites`) : Promise.resolve(null),
        canManage ? fetch(`/api/v1/packs/${packId}/deploy-tokens`) : Promise.resolve(null),
      ]);

      if (buildRes.ok) {
        const data = await buildRes.json();
        setBuilds(data.builds ?? []);
      }

      if (channelRes.ok) {
        const data = await channelRes.json();
        setChannels(data.channels ?? []);
      }

      if (memberRes.ok) {
        const data = await memberRes.json();
        setMembers(data.members ?? []);
      } else {
        setMembers([]);
      }

      if (inviteRes && inviteRes.ok) {
        const data = await inviteRes.json();
        setInvites(data.invites ?? []);
      } else {
        setInvites([]);
      }

      if (canManageRunnerTokens) {
        const tokenRes = await fetch(`/api/v1/runner/tokens?packId=${packId}`);
        if (tokenRes.ok) {
          const data = await tokenRes.json();
          setRunnerTokens(data.tokens ?? []);
        } else {
          setRunnerTokens([]);
        }
      }

      if (packDeployTokenRes && packDeployTokenRes.ok) {
        const data = await packDeployTokenRes.json();
        setPackDeployTokens(data.tokens ?? []);
      } else {
        setPackDeployTokens([]);
      }
    };

    loadDetails().catch(() => setError("Unable to load pack details."));
  }, [packId, canManage, canManageInvites, canManageRunnerTokens]);

  const handleCreateInvite = async () => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/invites`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create invite.");
      return;
    }

    const inviteCode = data?.invite?.code?.toString();
    const inviteUrl = data?.invite?.inviteUrl?.toString();
    if (inviteUrl) {
      setInviteLinkModal(inviteUrl);
    } else if (inviteCode) {
      setInviteLinkModal(`/invite?code=${inviteCode}`);
    }
    setInvites((prev) => [data.invite, ...prev]);
  };

  const handleDeleteInvite = async (inviteId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/invites`, {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ inviteId }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to delete invite.");
      return;
    }

    setInvites((prev) => prev.filter((invite) => invite.id !== inviteId));
  };

  const handlePromotion = async (channel: Channel["name"], buildId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/channels`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ channel, buildId }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update channel.");
      return;
    }

    setChannels((prev) =>
      prev.map((channel) =>
        channel.name === data.channel.name
          ? { ...channel, ...data.channel, buildId: data.channel.buildId }
          : channel
      )
    );
  };

  const handleToggleForceReinstall = async (buildId: string, forceReinstall: boolean) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/builds`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ buildId, forceReinstall }),
    });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to update build settings." }));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update build settings.");
      return;
    }

    const nextValue = Boolean(data?.build?.forceReinstall);
    setBuilds((prev) =>
      prev.map((build) =>
        build.id === buildId ? { ...build, forceReinstall: nextValue } : build
      )
    );
  };

  const handleRevokeMember = async (userId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/members/${userId}`, {
      method: "DELETE",
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to revoke access.");
      return;
    }

    setMembers((prev) => prev.filter((member) => member.userId !== userId));
  };

  const handleRevokeRunnerToken = async (tokenId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/runner/tokens`, {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ packId, tokenId }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to revoke runner token.");
      return;
    }

    setRunnerTokens((prev) =>
      prev.map((token) =>
        token.id === tokenId ? { ...token, revokedAt: data?.revokedAt ?? new Date().toISOString() } : token
      )
    );
  };

  const handleCreateRunnerToken = async () => {
    setLoading(true);
    setError(null);
    setCreatedRunnerToken(null);
    const response = await fetch(`/api/v1/runner/tokens`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        packId,
        name: runnerTokenName.trim() || null,
      }),
    });
    const data = await response.json().catch(() => ({}));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create runner token.");
      return;
    }

    setRunnerTokenName("");
    setCreatedRunnerToken(data?.token?.toString() ?? null);
    const nextPrefix = data?.prefix?.toString() ?? "";
    const nextId = data?.id?.toString() ?? `new-${Date.now()}`;
    setRunnerTokens((prev) => [
      {
        id: nextId,
        name: runnerTokenName.trim() || null,
        tokenPrefix: nextPrefix,
        createdAt: new Date().toISOString(),
        lastUsedAt: null,
        revokedAt: null,
        expiresAt: null,
      },
      ...prev,
    ]);
  };

  const handleCreatePackDeployToken = async () => {
    setLoading(true);
    setError(null);
    setCreatedPackDeployToken(null);
    const response = await fetch(`/api/v1/packs/${packId}/deploy-tokens`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: packDeployTokenName.trim() || null,
      }),
    });
    const data = await response.json().catch(() => ({}));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create pack deploy token.");
      return;
    }

    setPackDeployTokenName("");
    setCreatedPackDeployToken(data?.token?.toString() ?? null);
    const created = data?.record;
    if (created && typeof created.id === "string") {
      setPackDeployTokens((prev) => [created as PackDeployToken, ...prev]);
      return;
    }

    setPackDeployTokens((prev) => [
      {
        id: `new-${Date.now()}`,
        name: packDeployTokenName.trim() || null,
        tokenPrefix: "",
        createdAt: new Date().toISOString(),
        lastUsedAt: null,
        revokedAt: null,
        expiresAt: null,
      },
      ...prev,
    ]);
  };

  const handleRevokePackDeployToken = async (tokenId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/deploy-tokens`, {
      method: "DELETE",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tokenId }),
    });
    const data = await response.json().catch(() => ({}));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to revoke pack deploy token.");
      return;
    }

    setPackDeployTokens((prev) =>
      prev.map((token) =>
        token.id === tokenId
          ? { ...token, revokedAt: data?.revokedAt ?? new Date().toISOString() }
          : token
      )
    );
  };

  const handlePromoteMember = async (userId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/members/${userId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role: "creator" }),
    });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to promote member." }));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to promote member.");
      return;
    }

    setMembers((prev) =>
      prev.map((member) =>
        member.userId === userId
          ? { ...member, role: "creator", accessLevel: "all" }
          : member
      )
    );
  };

  const handleDemoteMember = async (userId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/members/${userId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role: "player" }),
    });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to demote member." }));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to demote member.");
      return;
    }

    setMembers((prev) =>
      prev.map((member) =>
        member.userId === userId
          ? { ...member, role: "player", accessLevel: "production" }
          : member
      )
    );
  };

  const handleUpdateAccessLevel = async (userId: string, accessLevel: AccessLevel) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}/members/${userId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ accessLevel }),
    });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to update access level." }));
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update access level.");
      return;
    }

    const nextAccess = (data?.member?.accessLevel ?? accessLevel) as AccessLevel;
    setMembers((prev) =>
      prev.map((member) =>
        member.userId === userId ? { ...member, accessLevel: nextAccess } : member
      )
    );
  };

  const handleDeletePack = async () => {
    if (!canDeletePack) {
      return;
    }

    const expected = packName.trim();
    if (!expected) {
      setError("Pack name is required before deletion.");
      return;
    }

    if (deleteConfirmation.trim() !== expected) {
      setError(`Type "${expected}" exactly to confirm deletion.`);
      return;
    }

    setDeletingPack(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}`, { method: "DELETE" });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to delete pack." }));
    setDeletingPack(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to delete pack.");
      return;
    }

    router.push("/dashboard");
    router.refresh();
  };

  const handleSaveFriendlyName = async () => {
    if (!canManageFriendlyName) {
      return;
    }

    const trimmed = friendlyName.trim();
    if (!trimmed) {
      setError("Friendly name cannot be empty.");
      return;
    }

    setSavingFriendlyName(true);
    setError(null);
    const response = await fetch(`/api/v1/packs/${packId}`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ friendlyName: trimmed }),
    });
    const data = await response
      .json()
      .catch(() => ({ error: "Unable to update friendly name." }));
    setSavingFriendlyName(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to update friendly name.");
      return;
    }

    setPack((prev) =>
      prev
        ? {
            ...prev,
            name: data.pack?.name ?? trimmed,
            updatedAt: data.pack?.updatedAt ?? prev.updatedAt,
          }
        : prev
    );
    setFriendlyName(data.pack?.name ?? trimmed);
  };

  return (
    <div className="min-h-screen bg-transparent px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <Tabs defaultValue="builds" className="space-y-6">
          <DashboardHeader
            workspaceName={pack?.name ?? packLabel}
            eyebrow="Pack"
            identifier={`ID: ${pack?.id ?? packId}`}
            leading={
              <Link href="/dashboard" aria-label="Back to all packs">
                <Button variant="outline" size="icon-sm">
                  <ArrowLeft className="h-4 w-4" />
                </Button>
              </Link>
            }
            actions={
              <>
                <SignOutButton />
              </>
            }
            tabs={
              <TabsList>
                <TabsTrigger value="builds">Builds</TabsTrigger>
                <TabsTrigger value="access">Access</TabsTrigger>
                <TabsTrigger value="manage">Manage</TabsTrigger>
              </TabsList>
            }
          />

          {error ? (
            <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
              {error}
            </div>
          ) : null}

          <TabsContent value="builds">
            <BuildsTab
              channels={channels}
              builds={builds}
              repoUrl={pack?.repoUrl}
              canPromoteBuilds={canPromoteBuilds}
              onPromote={handlePromotion}
              onToggleForceReinstall={handleToggleForceReinstall}
              loading={loading}
            />
          </TabsContent>

          <TabsContent value="access">
            <AccessTab
              canManageInvites={canManageInvites}
              invites={invites}
              onCreateInvite={handleCreateInvite}
              onDeleteInvite={handleDeleteInvite}
              inviteLinkModal={inviteLinkModal}
              onCloseInviteLinkModal={() => setInviteLinkModal(null)}
              members={members}
              onRevokeMember={handleRevokeMember}
              onPromoteMember={handlePromoteMember}
              onDemoteMember={handleDemoteMember}
              onUpdateAccessLevel={handleUpdateAccessLevel}
              loading={loading}
              currentUserId={session.user.id}
              canManageMembers={canManageMembers}
            />
          </TabsContent>

          <TabsContent value="manage">
              <ManageTab
                packName={packName}
                canManageFriendlyName={canManageFriendlyName}
                friendlyName={friendlyName}
                onFriendlyNameChange={setFriendlyName}
                savingFriendlyName={savingFriendlyName}
                onSaveFriendlyName={handleSaveFriendlyName}
                canDeletePack={canDeletePack}
                deleteConfirmation={deleteConfirmation}
                onDeleteConfirmationChange={setDeleteConfirmation}
                deletingPack={deletingPack}
                onDeletePack={handleDeletePack}
                runnerTokens={runnerTokens}
                canManageRunnerTokens={canManageRunnerTokens}
                runnerTokenName={runnerTokenName}
                onRunnerTokenNameChange={setRunnerTokenName}
                onCreateRunnerToken={handleCreateRunnerToken}
                createdRunnerToken={createdRunnerToken}
                onRevokeRunnerToken={handleRevokeRunnerToken}
                packDeployTokens={packDeployTokens}
                packDeployTokenName={packDeployTokenName}
                onPackDeployTokenNameChange={setPackDeployTokenName}
                onCreatePackDeployToken={handleCreatePackDeployToken}
                createdPackDeployToken={createdPackDeployToken}
                onRevokePackDeployToken={handleRevokePackDeployToken}
                loading={loading}
              />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
