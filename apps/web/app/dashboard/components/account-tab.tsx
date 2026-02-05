"use client";

import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";

interface AccountPasskey {
  id: string;
  name?: string;
  createdAt: string | Date;
  deviceType: string;
  backedUp: boolean;
}

interface AccountTabProps {
  onAddPasskey: (name?: string) => Promise<boolean>;
  onRenamePasskey: (id: string, name: string) => Promise<boolean>;
  onDeletePasskey: (id: string) => Promise<boolean>;
  passkeyLoading: boolean;
  passkeys: AccountPasskey[];
  passkeysLoading: boolean;
  passkeysError: string | null;
  githubLinked: boolean;
  githubLoading: boolean;
  githubError: string | null;
  onLinkGithub: () => void;
  onUnlinkGithub: () => void;
  focus: "github" | null;
  nextPath: string | null;
}

export default function AccountTab({
  onAddPasskey,
  onRenamePasskey,
  onDeletePasskey,
  passkeyLoading,
  passkeys,
  passkeysLoading,
  passkeysError,
  githubLinked,
  githubLoading,
  githubError,
  onLinkGithub,
  onUnlinkGithub,
  focus,
  nextPath,
}: AccountTabProps) {
  const [passkeyName, setPasskeyName] = useState("");
  const [editingPasskeyId, setEditingPasskeyId] = useState<string | null>(null);
  const [editingPasskeyName, setEditingPasskeyName] = useState("");
  const [mutatingPasskeyId, setMutatingPasskeyId] = useState<string | null>(null);
  const [passkeyActionError, setPasskeyActionError] = useState<string | null>(null);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setPasskeyActionError(null);
    const created = await onAddPasskey(passkeyName);
    if (created) {
      setPasskeyName("");
    }
  };

  const startRename = (id: string, currentName?: string) => {
    setPasskeyActionError(null);
    setEditingPasskeyId(id);
    setEditingPasskeyName(currentName?.trim() || "");
  };

  const cancelRename = () => {
    setEditingPasskeyId(null);
    setEditingPasskeyName("");
    setPasskeyActionError(null);
  };

  const saveRename = async (id: string) => {
    setPasskeyActionError(null);
    setMutatingPasskeyId(id);
    const updated = await onRenamePasskey(id, editingPasskeyName);
    setMutatingPasskeyId(null);

    if (updated) {
      setEditingPasskeyId(null);
      setEditingPasskeyName("");
      return;
    }

    setPasskeyActionError("Unable to rename passkey.");
  };

  const removePasskey = async (id: string) => {
    setPasskeyActionError(null);
    setMutatingPasskeyId(id);
    const deleted = await onDeletePasskey(id);
    setMutatingPasskeyId(null);

    if (!deleted) {
      setPasskeyActionError("Unable to delete passkey.");
      return;
    }

    if (editingPasskeyId === id) {
      setEditingPasskeyId(null);
      setEditingPasskeyName("");
    }
  };

  return (
    <div className="space-y-6">
      {focus === "github" ? (
        <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
          Connect a GitHub account to create new repositories. After linking, you&apos;ll
          be returned to {nextPath ?? "/dashboard/create"}.
        </div>
      ) : null}

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Passkeys</CardTitle>
            <CardDescription>
              Register a hardware-backed passkey for quick sign-in.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <form onSubmit={handleSubmit} className="space-y-3">
              <label className="block text-xs font-medium text-[var(--atlas-ink-muted)]">
                Passkey Name
                <Input
                  value={passkeyName}
                  onChange={(event) => setPasskeyName(event.target.value)}
                  placeholder="MacBook Pro"
                  maxLength={64}
                  className="mt-2"
                />
              </label>
              <Button type="submit" disabled={passkeyLoading}>
                {passkeyLoading ? "Waiting for device…" : "Add Passkey"}
              </Button>
            </form>

            <div className="space-y-2">
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-[var(--atlas-ink-muted)]">
                Active Passkeys
              </p>
              {passkeysError ? (
                <p className="rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
                  {passkeysError}
                </p>
              ) : null}
              {passkeyActionError ? (
                <p className="rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
                  {passkeyActionError}
                </p>
              ) : null}
              {passkeysLoading ? (
                <p className="text-xs text-[var(--atlas-ink-muted)]">Loading passkeys…</p>
              ) : null}
              {!passkeysLoading && !passkeysError && passkeys.length === 0 ? (
                <p className="text-xs text-[var(--atlas-ink-muted)]">
                  No passkeys registered yet.
                </p>
              ) : null}
              {!passkeysLoading && passkeys.length > 0
                ? passkeys.map((passkey) => (
                    <div
                      key={passkey.id}
                      className="rounded-2xl border border-[var(--atlas-border)] bg-[var(--atlas-cream)] px-3 py-2"
                    >
                      {editingPasskeyId === passkey.id ? (
                        <div className="space-y-2">
                          <Input
                            value={editingPasskeyName}
                            onChange={(event) => setEditingPasskeyName(event.target.value)}
                            placeholder="Passkey name"
                            maxLength={64}
                          />
                          <div className="flex flex-wrap gap-2">
                            <Button
                              size="sm"
                              onClick={() => saveRename(passkey.id)}
                              disabled={mutatingPasskeyId === passkey.id}
                            >
                              {mutatingPasskeyId === passkey.id ? "Saving…" : "Save"}
                            </Button>
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={cancelRename}
                              disabled={mutatingPasskeyId === passkey.id}
                            >
                              Cancel
                            </Button>
                          </div>
                        </div>
                      ) : (
                        <div className="flex items-start justify-between gap-3">
                          <p className="text-sm font-semibold text-[var(--atlas-ink)]">
                            {passkey.name?.trim() || "Unnamed passkey"}
                          </p>
                          <div className="flex gap-2">
                            <Button
                              size="sm"
                              variant="outline"
                              onClick={() => startRename(passkey.id, passkey.name)}
                              disabled={mutatingPasskeyId === passkey.id}
                            >
                              Rename
                            </Button>
                            <Button
                              size="sm"
                              variant="destructive"
                              onClick={() => removePasskey(passkey.id)}
                              disabled={mutatingPasskeyId === passkey.id}
                            >
                              {mutatingPasskeyId === passkey.id ? "Deleting…" : "Delete"}
                            </Button>
                          </div>
                        </div>
                      )}
                      <p className="text-xs text-[var(--atlas-ink-muted)]">
                        {passkey.deviceType}
                        {" · "}
                        Added{" "}
                        {new Date(passkey.createdAt).toLocaleDateString(undefined, {
                          year: "numeric",
                          month: "short",
                          day: "numeric",
                        })}
                        {" · "}
                        {passkey.backedUp ? "Synced" : "Local only"}
                      </p>
                    </div>
                  ))
                : null}
            </div>
          </CardContent>
        </Card>

        <Card className={focus === "github" ? "ring-2 ring-amber-200" : ""}>
          <CardHeader>
            <CardTitle>GitHub</CardTitle>
            <CardDescription>
              Link GitHub to create repositories and pull org ownership.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-sm font-semibold">
                  {githubLinked ? "Connected" : "Not connected"}
                </p>
                <p className="text-xs text-[var(--atlas-ink-muted)]">
                  {githubLinked
                    ? "GitHub is linked to your account."
                    : "Link GitHub to continue creating pack repositories."}
                </p>
              </div>
              {githubLinked ? (
                <Button
                  variant="outline"
                  onClick={onUnlinkGithub}
                  disabled={githubLoading}
                >
                  Disconnect
                </Button>
              ) : (
                <Button onClick={onLinkGithub} disabled={githubLoading}>
                  Connect GitHub
                </Button>
              )}
            </div>
            {githubError ? (
              <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-2 text-xs text-red-700">
                {githubError}
              </div>
            ) : null}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
