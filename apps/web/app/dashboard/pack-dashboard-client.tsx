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
  ApiKey,
  Build,
  Channel,
  Invite,
  Pack,
  PackMember,
  Role,
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
  const [apiKeyRecords, setApiKeyRecords] = useState<ApiKey[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [apiKeyLabel, setApiKeyLabel] = useState("");
  const [newApiKey, setNewApiKey] = useState<string | null>(null);
  const [inviteLinkModal, setInviteLinkModal] = useState<string | null>(null);
  const [friendlyName, setFriendlyName] = useState("");
  const [savingFriendlyName, setSavingFriendlyName] = useState(false);
  const [deleteConfirmation, setDeleteConfirmation] = useState("");
  const [deletingPack, setDeletingPack] = useState(false);

  const canManage = session.user.role === "admin" || session.user.role === "creator";
  const canPromoteBuilds = canManage;
  const canManageInvites = canManage;
  const canManageMembers = canManage;
  const canManageApiKeys = canManage;
  const canManageFriendlyName = canManage;
  const canDeletePack = canManage;

  const packLabel = useMemo(() => pack?.slug ?? pack?.name ?? packId, [pack, packId]);
  const packName = pack?.name ?? packId;

  useEffect(() => {
    const loadPack = async () => {
      const response = await fetch("/api/packs");
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
      const [buildRes, channelRes, memberRes, inviteRes, tokenRes] = await Promise.all([
        fetch(`/api/packs/${packId}/builds`),
        fetch(`/api/packs/${packId}/channels`),
        fetch(`/api/packs/${packId}/members`),
        canManageInvites ? fetch(`/api/packs/${packId}/invites`) : Promise.resolve(null),
        canManageApiKeys ? fetch(`/api/packs/${packId}/api-keys`) : Promise.resolve(null),
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

      if (tokenRes && tokenRes.ok) {
        const data = await tokenRes.json();
        setApiKeyRecords(data.keys ?? []);
      } else {
        setApiKeyRecords([]);
      }
    };

    loadDetails().catch(() => setError("Unable to load pack details."));
  }, [packId, canManageInvites, canManageApiKeys]);

  const handleCreateInvite = async () => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${packId}/invites`, {
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
    const response = await fetch(`/api/packs/${packId}/invites`, {
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
    const response = await fetch(`/api/packs/${packId}/channels`, {
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

  const handleCreateApiKey = async () => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${packId}/api-keys`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ label: apiKeyLabel || undefined }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create deploy key.");
      return;
    }

    setApiKeyLabel("");
    setNewApiKey(data.key);
    setApiKeyRecords((prev) => [data.record, ...prev]);
  };

  const handleRevokeMember = async (userId: string) => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${packId}/members/${userId}`, {
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
    const response = await fetch(`/api/packs/${packId}`, { method: "DELETE" });
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
    const response = await fetch(`/api/packs/${packId}`, {
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
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <Tabs defaultValue="builds" className="space-y-6">
          <DashboardHeader
            workspaceName={pack?.name ?? packLabel}
            eyebrow="Pack"
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
              canPromoteBuilds={canPromoteBuilds}
              onPromote={handlePromotion}
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
              canManageApiKeys={canManageApiKeys}
              apiKeyRecords={apiKeyRecords}
              apiKeyLabel={apiKeyLabel}
              newApiKey={newApiKey}
              onApiKeyLabelChange={setApiKeyLabel}
              onCreateApiKey={handleCreateApiKey}
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
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
