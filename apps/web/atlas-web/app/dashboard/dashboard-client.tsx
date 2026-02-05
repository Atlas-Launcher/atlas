"use client";

import { useEffect, useMemo, useState } from "react";

import { authClient } from "@/lib/auth-client";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Separator } from "@/components/ui/separator";
import SignOutButton from "@/app/dashboard/sign-out-button";
interface DashboardClientProps {
  session: {
    user: {
      id: string;
      email: string;
      role: "admin" | "creator" | "player";
    };
  };
}

interface Pack {
  id: string;
  name: string;
  slug: string;
  description?: string | null;
  repoUrl?: string | null;
  createdAt?: string;
  updatedAt?: string;
}

interface Build {
  id: string;
  version: string;
  commitHash?: string | null;
  artifactKey: string;
  createdAt?: string;
}

interface Channel {
  id: string;
  name: "dev" | "beta" | "production";
  buildId?: string | null;
  buildVersion?: string | null;
  buildCommit?: string | null;
  updatedAt?: string;
}

interface Invite {
  id: string;
  email?: string | null;
  code: string;
  role: "admin" | "creator" | "player";
  accessLevel: "dev" | "beta" | "production";
  usedAt?: string | null;
  createdAt?: string;
}

interface DeployToken {
  id: string;
  label?: string | null;
  active: boolean;
  createdAt?: string;
  revokedAt?: string | null;
}

export default function DashboardClient({ session }: DashboardClientProps) {
  const [packs, setPacks] = useState<Pack[]>([]);
  const [selectedPackId, setSelectedPackId] = useState<string | null>(null);
  const [builds, setBuilds] = useState<Build[]>([]);
  const [channels, setChannels] = useState<Channel[]>([]);
  const [invites, setInvites] = useState<Invite[]>([]);
  const [deployTokenRecords, setDeployTokenRecords] = useState<DeployToken[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const [packName, setPackName] = useState("");
  const [packSlug, setPackSlug] = useState("");
  const [packRepoUrl, setPackRepoUrl] = useState("");

  const [inviteEmail, setInviteEmail] = useState("");
  const [inviteRole, setInviteRole] = useState("player");
  const [inviteAccess, setInviteAccess] = useState("production");
  const [redeemCode, setRedeemCode] = useState("");

  const [promotionChannel, setPromotionChannel] = useState("dev");
  const [promotionBuild, setPromotionBuild] = useState("");
  const [deployTokenLabel, setDeployTokenLabel] = useState("");
  const [newDeployToken, setNewDeployToken] = useState<string | null>(null);

  const selectedPack = useMemo(
    () => packs.find((pack) => pack.id === selectedPackId) ?? null,
    [packs, selectedPackId]
  );

  const canCreatePack = session.user.role === "admin" || session.user.role === "creator";

  useEffect(() => {
    const loadPacks = async () => {
      setLoading(true);
      const response = await fetch("/api/packs");
      const data = await response.json();
      setLoading(false);

      if (!response.ok) {
        setError(data?.error ?? "Unable to load packs.");
        return;
      }

      setPacks(data.packs ?? []);
      if (!selectedPackId && data.packs?.length) {
        setSelectedPackId(data.packs[0].id);
      }
    };

    loadPacks().catch(() => setError("Unable to load packs."));
  }, [selectedPackId]);

  useEffect(() => {
    if (!selectedPackId) {
      setBuilds([]);
      setChannels([]);
      setInvites([]);
      setDeployTokenRecords([]);
      setNewDeployToken(null);
      return;
    }

    const loadDetails = async () => {
      const [buildRes, channelRes, inviteRes, tokenRes] = await Promise.all([
        fetch(`/api/packs/${selectedPackId}/builds`),
        fetch(`/api/packs/${selectedPackId}/channels`),
        fetch(`/api/packs/${selectedPackId}/invites`),
        fetch(`/api/packs/${selectedPackId}/deploy-tokens`),
      ]);

      if (buildRes.ok) {
        const data = await buildRes.json();
        setBuilds(data.builds ?? []);
      }

      if (channelRes.ok) {
        const data = await channelRes.json();
        setChannels(data.channels ?? []);
      }

      if (inviteRes.ok) {
        const data = await inviteRes.json();
        setInvites(data.invites ?? []);
      }

      if (tokenRes.ok) {
        const data = await tokenRes.json();
        setDeployTokenRecords(data.tokens ?? []);
      }
    };

    setNewDeployToken(null);
    loadDetails().catch(() => setError("Unable to load pack details."));
  }, [selectedPackId]);

  const handleCreatePack = async () => {
    if (!packName) {
      setError("Pack name is required.");
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch("/api/packs", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: packName,
        slug: packSlug || undefined,
        repoUrl: packRepoUrl || undefined,
      }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create pack.");
      return;
    }

    setPackName("");
    setPackSlug("");
    setPackRepoUrl("");
    setPacks((prev) => [data.pack, ...prev]);
    setSelectedPackId(data.pack.id);
  };

  const handleCreateInvite = async () => {
    if (!selectedPackId) {
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${selectedPackId}/invites`, {
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

  const handleRedeemInvite = async () => {
    if (!redeemCode.trim()) {
      setError("Enter an invite code to join a pack.");
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch("/api/invites/accept", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code: redeemCode.trim() }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to redeem invite.");
      return;
    }

    setRedeemCode("");
    const packsResponse = await fetch("/api/packs");
    if (packsResponse.ok) {
      const packsData = await packsResponse.json();
      setPacks(packsData.packs ?? []);
    }
  };

  const handlePromotion = async () => {
    if (!selectedPackId || !promotionBuild) {
      setError("Choose a build to promote.");
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${selectedPackId}/channels`, {
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

  const handleAddPasskey = async () => {
    const result = await authClient.passkey.addPasskey();
    if (result?.error) {
      setError(result.error.message ?? "Unable to add passkey.");
      return;
    }
  };

  const handleCreateDeployToken = async () => {
    if (!selectedPackId) {
      return;
    }

    setLoading(true);
    setError(null);
    const response = await fetch(`/api/packs/${selectedPackId}/deploy-tokens`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ label: deployTokenLabel || undefined }),
    });
    const data = await response.json();
    setLoading(false);

    if (!response.ok) {
      setError(data?.error ?? "Unable to create deploy token.");
      return;
    }

    setDeployTokenLabel("");
    setNewDeployToken(data.token);
    setDeployTokenRecords((prev) => [data.record, ...prev]);
  };

  const handleOpenDeviceFlow = () => {
    window.location.href = "/device";
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <Card>
          <CardHeader>
            <Badge variant="secondary">Atlas Hub</Badge>
            <CardTitle>Welcome back</CardTitle>
            <CardDescription>
              Signed in as {session.user.email} ({session.user.role}).
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex gap-3">
              <Badge>{packs.length} Packs</Badge>
              <Badge variant="outline">Active: {selectedPack?.slug ?? "None"}</Badge>
            </div>
            <SignOutButton />
          </CardContent>
        </Card>

        {error ? (
          <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
            {error}
          </div>
        ) : null}

        <Tabs defaultValue="overview">
          <TabsList>
            <TabsTrigger value="overview">Overview</TabsTrigger>
            <TabsTrigger value="packs">Packs</TabsTrigger>
            <TabsTrigger value="invites">Invites</TabsTrigger>
            <TabsTrigger value="security">Security</TabsTrigger>
          </TabsList>

          <TabsContent value="overview">
            <div className="grid gap-6 md:grid-cols-2">
              <Card>
                <CardHeader>
                  <CardTitle>Release Channels</CardTitle>
                  <CardDescription>Immutable builds, mutable pointers.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-3">
                  {channels.length ? (
                    channels.map((channel) => (
                      <div
                        key={channel.id}
                        className="rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/70 px-4 py-3"
                      >
                        <div className="flex items-center justify-between text-sm font-semibold">
                          <span>{channel.name.toUpperCase()}</span>
                          <Badge variant="secondary">Live</Badge>
                        </div>
                        <p className="mt-2 text-xs text-[var(--atlas-ink-muted)]">
                          {channel.buildVersion ?? "No build"} {channel.buildCommit ? `(${channel.buildCommit})` : ""}
                        </p>
                      </div>
                    ))
                  ) : (
                    <p className="text-sm text-[var(--atlas-ink-muted)]">Select a pack to view channels.</p>
                  )}
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Promote Build</CardTitle>
                  <CardDescription>Move a channel pointer to a new build.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                    Channel
                    <select
                      value={promotionChannel}
                      onChange={(event) => setPromotionChannel(event.target.value)}
                      className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                    >
                      <option value="dev">Dev</option>
                      <option value="beta">Beta</option>
                      <option value="production">Production</option>
                    </select>
                  </label>
                  <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                    Build
                    <select
                      value={promotionBuild}
                      onChange={(event) => setPromotionBuild(event.target.value)}
                      className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                    >
                      <option value="">Select build</option>
                      {builds.map((build) => (
                        <option key={build.id} value={build.id}>
                          {build.version} {build.commitHash ? `(${build.commitHash})` : ""}
                        </option>
                      ))}
                    </select>
                  </label>
                  <Button onClick={handlePromotion} disabled={loading}>
                    Promote
                  </Button>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="packs">
            <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
              <Card>
                <CardHeader>
                  <CardTitle>Pack Library</CardTitle>
                  <CardDescription>Select a pack to see builds and channels.</CardDescription>
                </CardHeader>
                <CardContent>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Name</TableHead>
                        <TableHead>Slug</TableHead>
                        <TableHead>Repo</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {packs.map((pack) => (
                        <TableRow
                          key={pack.id}
                          className={`cursor-pointer ${
                            pack.id === selectedPackId ? "bg-[var(--atlas-cream)]/70" : ""
                          }`}
                          onClick={() => setSelectedPackId(pack.id)}
                        >
                          <TableCell className="font-semibold">{pack.name}</TableCell>
                          <TableCell>{pack.slug}</TableCell>
                          <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                            {pack.repoUrl ?? "-"}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Create Pack</CardTitle>
                  <CardDescription>Creator/admin only.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {canCreatePack ? (
                    <>
                      <Input
                        placeholder="Pack name"
                        value={packName}
                        onChange={(event) => setPackName(event.target.value)}
                      />
                      <Input
                        placeholder="Slug (optional)"
                        value={packSlug}
                        onChange={(event) => setPackSlug(event.target.value)}
                      />
                      <Input
                        placeholder="GitHub repo URL (optional)"
                        value={packRepoUrl}
                        onChange={(event) => setPackRepoUrl(event.target.value)}
                      />
                      <Button onClick={handleCreatePack} disabled={loading}>
                        Create Pack
                      </Button>
                    </>
                  ) : (
                    <p className="text-sm text-[var(--atlas-ink-muted)]">
                      You need creator or admin access to create packs.
                    </p>
                  )}
                </CardContent>
              </Card>
            </div>

            <Separator className="my-6" />

            <Card>
              <CardHeader>
                <CardTitle>Build Ledger</CardTitle>
                <CardDescription>Immutable builds received from CI.</CardDescription>
              </CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Version</TableHead>
                      <TableHead>Commit</TableHead>
                      <TableHead>Artifact</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {builds.map((build) => (
                      <TableRow key={build.id}>
                        <TableCell className="font-semibold">{build.version}</TableCell>
                        <TableCell>{build.commitHash ?? "-"}</TableCell>
                        <TableCell className="text-xs text-[var(--atlas-ink-muted)]">
                          {build.artifactKey}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="invites">
            <div className="grid gap-6 lg:grid-cols-[1fr_0.9fr]">
              <Card>
                <CardHeader>
                  <CardTitle>Invite Ledger</CardTitle>
                  <CardDescription>Share access links with players or creators.</CardDescription>
                </CardHeader>
                <CardContent>
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>Code</TableHead>
                        <TableHead>Role</TableHead>
                        <TableHead>Access</TableHead>
                        <TableHead>Status</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {invites.map((invite) => (
                        <TableRow key={invite.id}>
                          <TableCell className="font-semibold">{invite.code}</TableCell>
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

              <Card>
                <CardHeader>
                  <CardTitle>Create Invite</CardTitle>
                  <CardDescription>Generates a new code instantly.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <Input
                    placeholder="Email (optional)"
                    value={inviteEmail}
                    onChange={(event) => setInviteEmail(event.target.value)}
                  />
                  <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                    Role
                    <select
                      value={inviteRole}
                      onChange={(event) => setInviteRole(event.target.value)}
                      className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                    >
                      <option value="player">Player</option>
                      <option value="creator">Creator</option>
                    </select>
                  </label>
                  <label className="block text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                    Access Level
                    <select
                      value={inviteAccess}
                      onChange={(event) => setInviteAccess(event.target.value)}
                      className="mt-2 h-12 w-full rounded-2xl border border-[var(--atlas-ink)]/20 bg-white px-4 text-sm"
                    >
                      <option value="production">Production</option>
                      <option value="beta">Beta</option>
                      <option value="dev">Dev</option>
                    </select>
                  </label>
                  <Button onClick={handleCreateInvite} disabled={loading || !selectedPackId}>
                    Create Invite
                  </Button>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Redeem Invite</CardTitle>
                  <CardDescription>Join a pack with a creator-issued code.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  <Input
                    placeholder="Invite code"
                    value={redeemCode}
                    onChange={(event) => setRedeemCode(event.target.value)}
                  />
                  <Button variant="outline" onClick={handleRedeemInvite} disabled={loading}>
                    Join Pack
                  </Button>
                </CardContent>
              </Card>
            </div>
          </TabsContent>

          <TabsContent value="security">
            <div className="grid gap-6 lg:grid-cols-3">
              <Card>
                <CardHeader>
                  <CardTitle>Passkeys</CardTitle>
                  <CardDescription>
                    Register a hardware-backed passkey for quick sign-in.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <Button onClick={handleAddPasskey}>Add Passkey</Button>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Device Login</CardTitle>
                  <CardDescription>
                    Enter a launcher device code to authorize the session.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <Button variant="outline" onClick={handleOpenDeviceFlow}>
                    Open Device Flow
                  </Button>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle>Deploy Tokens</CardTitle>
                  <CardDescription>Used by CI to upload new builds.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {newDeployToken ? (
                    <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
                      Copy this token now: {newDeployToken}
                    </div>
                  ) : null}
                  <Input
                    placeholder="Label (optional)"
                    value={deployTokenLabel}
                    onChange={(event) => setDeployTokenLabel(event.target.value)}
                  />
                  <Button onClick={handleCreateDeployToken} disabled={loading || !selectedPackId}>
                    Generate Token
                  </Button>
                  <div className="space-y-2 text-xs text-[var(--atlas-ink-muted)]">
                    {deployTokenRecords.length ? (
                      deployTokenRecords.map((token) => (
                        <div
                          key={token.id}
                          className="flex items-center justify-between rounded-2xl border border-[var(--atlas-ink)]/10 bg-[var(--atlas-cream)]/60 px-3 py-2"
                        >
                          <span>{token.label || "Deploy token"}</span>
                          <Badge variant={token.active ? "secondary" : "outline"}>
                            {token.active ? "Active" : "Revoked"}
                          </Badge>
                        </div>
                      ))
                    ) : (
                      <span>No deploy tokens yet.</span>
                    )}
                  </div>
                </CardContent>
              </Card>
            </div>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
