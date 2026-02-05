"use client";

import { useEffect, useMemo, useState } from "react";

import BuildsTab from "@/app/dashboard/components/builds-tab";
import AccessTab from "@/app/dashboard/components/access-tab";
import PackHeader from "@/app/dashboard/components/pack-header";
import SignOutButton from "@/app/dashboard/sign-out-button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { ApiKey, Build, Channel, Invite, Pack, Role } from "@/app/dashboard/types";

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
  const [pack, setPack] = useState<Pack | null>(null);
  const [builds, setBuilds] = useState<Build[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [invites, setInvites] = useState<Invite[]>([]);
  const [apiKeyRecords, setApiKeyRecords] = useState<ApiKey[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState("player");
  const [inviteAccess, setInviteAccess] = useState("production");
  const [promotionChannel, setPromotionChannel] = useState("dev");
  const [promotionBuild, setPromotionBuild] = useState("");
  const [apiKeyLabel, setApiKeyLabel] = useState("");
  const [newApiKey, setNewApiKey] = useState<string | null>(null);

  const canManage = session.user.role === "admin" || session.user.role === "creator";
  const canPromoteBuilds = canManage;
  const canManageInvites = canManage;
  const canManageApiKeys = canManage;

  const packLabel = useMemo(() => pack?.slug ?? pack?.name ?? packId, [pack, packId]);

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
      if (!found) {
        setError("You do not have access to this pack.");
      }
    };

    loadPack().catch(() => setError("Unable to load pack."));
  }, [packId]);

  useEffect(() => {
    const loadDetails = async () => {
      const requests = [
        fetch(`/api/packs/${packId}/builds`),
        fetch(`/api/packs/${packId}/channels`),
      ];

      if (canManageInvites) {
        requests.push(fetch(`/api/packs/${packId}/invites`));
      }

      if (canManageApiKeys) {
        requests.push(fetch(`/api/packs/${packId}/api-keys`));
      }

      const responses = await Promise.all(requests);
      const buildRes = responses[0];
      const channelRes = responses[1];
      const inviteRes = canManageInvites ? responses[2] : null;
      const tokenRes = canManageApiKeys
        ? responses[canManageInvites ? 3 : 2]
        : null;

      if (buildRes.ok) {
        const data = await buildRes.json();
        setBuilds(data.builds ?? []);
      }

      if (channelRes.ok) {
        const data = await channelRes.json();
        setChannels(data.channels ?? []);
      }

      if (inviteRes && inviteRes.ok) {
        const data = await inviteRes.json();
        setInvites(data.invites ?? []);
      } else if (!canManageInvites) {
        setInvites([]);
      }

      if (tokenRes && tokenRes.ok) {
        const data = await tokenRes.json();
        setApiKeyRecords(data.keys ?? []);
      } else if (!canManageApiKeys) {
        setApiKeyRecords([]);
      }
    };

    setNewApiKey(null);
    loadDetails().catch(() => setError("Unable to load pack details."));
  }, [packId, canManageInvites, canManageApiKeys]);

  const handleCreateInvite = async () => {
    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${packId}/invites`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: inviteEmail || undefined,
        role: inviteRole,
        accessLevel: inviteAccess,
      }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create invite.");
      return;
    }

    setInviteEmail("");
    setInvites((prev) => [data.invite, ...prev]);
  };

  const handlePromotion = async () => {
    if (!promotionBuild) {
      setError("Choose a build to promote.");
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${packId}/channels`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ channel: promotionChannel, buildId: promotionBuild }),
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

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <PackHeader
          name={pack?.name ?? packLabel}
          slug={pack?.slug}
          packId={packId}
          actions={<SignOutButton />}
        />

        {error ? (
          <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
            {error}
          </div>
        ) : null}

        <Tabs defaultValue="builds">
          <TabsList>
            <TabsTrigger value="builds">Builds</TabsTrigger>
            <TabsTrigger value="access">Access</TabsTrigger>
          </TabsList>

          <TabsContent value="builds">
            <BuildsTab
              channels={channels}
              builds={builds}
              canPromoteBuilds={canPromoteBuilds}
              promotionChannel={promotionChannel}
              promotionBuild={promotionBuild}
              onPromotionChannelChange={setPromotionChannel}
              onPromotionBuildChange={setPromotionBuild}
              onPromote={handlePromotion}
              loading={loading}
            />
          </TabsContent>

          <TabsContent value="access">
            <AccessTab
              canManageInvites={canManageInvites}
              invites={invites}
              inviteEmail={inviteEmail}
              inviteRole={inviteRole}
              inviteAccess={inviteAccess}
              onInviteEmailChange={setInviteEmail}
              onInviteRoleChange={setInviteRole}
              onInviteAccessChange={setInviteAccess}
              onCreateInvite={handleCreateInvite}
              canManageApiKeys={canManageApiKeys}
              apiKeyRecords={apiKeyRecords}
              apiKeyLabel={apiKeyLabel}
              newApiKey={newApiKey}
              onApiKeyLabelChange={setApiKeyLabel}
              onCreateApiKey={handleCreateApiKey}
              loading={loading}
              selectedPackId={packId}
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
