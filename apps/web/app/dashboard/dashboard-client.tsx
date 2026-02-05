"use client";

import { useEffect, useState } from "react";

import { useRouter, useSearchParams } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import SignOutButton from "@/app/dashboard/sign-out-button";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import PacksTab from "@/app/dashboard/components/packs-tab";
import AccountTab from "@/app/dashboard/components/account-tab";
import SystemTab from "@/app/dashboard/components/system-tab";
import type { Pack, Role } from "@/app/dashboard/types";

interface DashboardClientProps {
  session: {
    user: {
      id: string;
      email: string;
      role: Role;
    };
  };
}

export default function DashboardClient({ session }: DashboardClientProps) {
  const router = useRouter();
  const searchParams = useSearchParams();
  const isAdmin = session.user.role === "admin";
  const initialTabParam = searchParams.get("tab");
  const initialTab =
    initialTabParam === "account"
      ? "account"
      : isAdmin && initialTabParam === "system"
        ? "system"
        : "overview";
  const focus = searchParams.get("focus");
  const nextPath = searchParams.get("next");
  const [packs, setPacks] = useState<Pack[]>([]);
  const [selectedPackId, setSelectedPackId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [tabValue, setTabValue] = useState<"overview" | "account" | "system">(
    initialTab
  );
  const [githubLinked, setGithubLinked] = useState(false);
  const [githubLoading, setGithubLoading] = useState(false);
  const [githubError, setGithubError] = useState<string | null>(null);

  const canManage = session.user.role === "admin" || session.user.role === "creator";
  const canCreatePack = canManage;
  const handleSelectPack = (packId: string) => {
    setSelectedPackId(packId);
    router.push(`/dashboard/${packId}`);
  };

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
    setTabValue(initialTab);
  }, [initialTab]);

  useEffect(() => {
    const loadAccounts = async () => {
      setGithubLoading(true);
      setGithubError(null);
      const response = await fetch("/api/auth/list-accounts");
      const data = await response.json();
      setGithubLoading(false);

      if (!response.ok) {
        setGithubError(data?.error ?? "Unable to load linked accounts.");
        return;
      }

      const github = (data ?? []).find(
        (account: { providerId?: string }) => account.providerId === "github"
      );
      setGithubLinked(Boolean(github));
    };

    loadAccounts().catch(() => {
      setGithubLoading(false);
      setGithubError("Unable to load linked accounts.");
    });
  }, []);

  const handleAddPasskey = async () => {
    const result = await authClient.passkey.addPasskey();
    if (result?.error) {
      setError(result.error.message ?? "Unable to add passkey.");
      return;
    }
  };

  const handleLinkGithub = async () => {
    setGithubError(null);
    setGithubLoading(true);
    const callbackURL = new URL(
      nextPath ?? "/dashboard?tab=account",
      window.location.origin
    ).toString();

    const response = await fetch("/api/auth/link-social", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        provider: "github",
        callbackURL,
        scopes: ["repo", "read:org", "user:email"],
        disableRedirect: true,
      }),
    });
    const data = await response.json();
    setGithubLoading(false);

    if (!response.ok) {
      setGithubError(data?.error ?? "Unable to link GitHub.");
      return;
    }

    if (data?.url) {
      window.location.href = data.url;
    } else {
      setGithubError("Unable to start GitHub linking.");
    }
  };

  const handleUnlinkGithub = async () => {
    setGithubError(null);
    setGithubLoading(true);
    const response = await fetch("/api/auth/unlink-account", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ providerId: "github" }),
    });
    const data = await response.json();
    setGithubLoading(false);

    if (!response.ok) {
      setGithubError(data?.error ?? "Unable to unlink GitHub.");
      return;
    }

    setGithubLinked(false);
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <Tabs value={tabValue} onValueChange={(value) => setTabValue(value as typeof tabValue)} className="space-y-6">
          <DashboardHeader
            workspaceName="Atlas Hub"
            email={session.user.email}
            role={session.user.role}
            actions={<SignOutButton />}
            tabs={
              <TabsList className="w-full justify-start">
                <TabsTrigger value="overview">Overview</TabsTrigger>
                <TabsTrigger value="account">Account</TabsTrigger>
                {isAdmin ? <TabsTrigger value="system">System</TabsTrigger> : null}
              </TabsList>
            }
          />

          {error ? (
            <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
              {error}
            </div>
          ) : null}

          <TabsContent value="overview">
            <PacksTab
              packs={packs}
              selectedPackId={selectedPackId}
              onSelectPack={handleSelectPack}
              canCreatePack={canCreatePack}
            />
          </TabsContent>

          <TabsContent value="account">
            <AccountTab
              onAddPasskey={handleAddPasskey}
              githubLinked={githubLinked}
              githubLoading={githubLoading}
              githubError={githubError}
              onLinkGithub={handleLinkGithub}
              onUnlinkGithub={handleUnlinkGithub}
              focus={focus === "github" ? "github" : null}
              nextPath={nextPath}
            />
          </TabsContent>

          {isAdmin ? (
            <TabsContent value="system">
              <SystemTab />
            </TabsContent>
          ) : null}
        </Tabs>
      </div>
    </div>
  );
}
