"use client";

import { useEffect, useState } from "react";

import { useRouter } from "next/navigation";

import { authClient } from "@/lib/auth-client";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import SignOutButton from "@/app/dashboard/sign-out-button";
import DashboardHeader from "@/app/dashboard/components/dashboard-header";
import PacksTab from "@/app/dashboard/components/packs-tab";
import AccountTab from "@/app/dashboard/components/account-tab";
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
  const [packs, setPacks] = useState<Pack[]>([]);
  const [selectedPackId, setSelectedPackId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

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

  const handleAddPasskey = async () => {
    const result = await authClient.passkey.addPasskey();
    if (result?.error) {
      setError(result.error.message ?? "Unable to add passkey.");
      return;
    }
  };

  const handleOpenDeviceFlow = () => {
    window.location.href = "/device";
  };

  return (
    <div className="min-h-screen bg-[var(--atlas-cream)] px-6 py-12 text-[var(--atlas-ink)]">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        <Tabs defaultValue="overview" className="space-y-6">
          <DashboardHeader
            workspaceName="Atlas Hub"
            email={session.user.email}
            role={session.user.role}
            actions={<SignOutButton />}
            tabs={
              <TabsList className="w-full justify-start">
                <TabsTrigger value="overview">Overview</TabsTrigger>
                <TabsTrigger value="account">Account</TabsTrigger>
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
              onOpenDeviceFlow={handleOpenDeviceFlow}
            />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
