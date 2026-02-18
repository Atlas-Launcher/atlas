"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { ArrowLeft, Github } from "lucide-react";
import { useForm } from "@tanstack/react-form";

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
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { RepositorySelector } from "./repository-selector";

function deriveNameFromRepoUrl(value: string) {
  const trimmed = value.trim().replace(/\.git$/, "");
  if (!trimmed) {
    return "";
  }
  const pieces = trimmed.split("/");
  const repo = pieces[pieces.length - 1] ?? "";
  if (!repo) {
    return "";
  }
  return repo.replace(/[-_]+/g, " ").trim();
}

const requireValue =
  (label: string) =>
    ({ value }: { value: string }) =>
      value?.trim() ? undefined : `${label} is required.`;

const GITHUB_APP_SLUG = process.env.NEXT_PUBLIC_GITHUB_APP_SLUG || "my-atlas-app";

export default function CreatePackClient() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [errorCode, setErrorCode] = useState<string | null>(null);
  const [installUrl, setInstallUrl] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState("import");
  const [repoSelectorOpen, setRepoSelectorOpen] = useState(false);

  const createPack = async (payload: {
    name: string;
    slug?: string;
    repoUrl?: string;
  }) => {
    const response = await fetch("/api/v1/packs", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        name: payload.name.trim(),
        slug: payload.slug?.trim() || undefined,
        repoUrl: payload.repoUrl?.trim() || undefined,
      }),
    });
    const data = await response.json();
    return { ok: response.ok, data };
  };

  const handleCreatePack = async (payload: {
    name: string;
    slug?: string;
    repoUrl?: string;
  }) => {
    if (!payload.name.trim()) {
      setError("Pack name is required.");
      return;
    }

    setLoading(true);
    setError(null);
    setErrorCode(null);
    setInstallUrl(null);
    const result = await createPack(payload);
    setLoading(false);

    if (!result.ok) {
      setError(result.data?.error ?? "Unable to create pack.");
      setErrorCode(result.data?.code ?? null);
      setInstallUrl(result.data?.installUrl ?? null);
      return;
    }

    router.push(`/dashboard/${result.data.pack.id}`);
  };

  const importForm = useForm({
    defaultValues: {
      owner: "",
      repoUrl: "",
      name: "",
      slug: "",
    },
    onSubmit: async ({ value }) => {
      await handleCreatePack({
        name: value.name,
        slug: value.slug,
        repoUrl: value.repoUrl,
      });
    },
  });

  const newRepoForm = useForm({
    defaultValues: {
      owner: "",
      name: "",
      description: "",
      visibility: "private",
    },
    onSubmit: async ({ value }) => {
      setLoading(true);
      setError(null);
      setInstallUrl(null);

      const response = await fetch("/api/v1/github/repos", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          owner: value.owner,
          name: value.name,
          description: value.description || undefined,
          visibility: value.visibility,
        }),
      });
      const data = await response.json();

      if (!response.ok) {
        setError(data?.error ?? "Unable to create GitHub repository.");
        setErrorCode(data?.code ?? null);
        setInstallUrl(data?.installUrl ?? null);
        setLoading(false);
        return;
      }
      setLoading(false);
      router.push(`/dashboard/${data.pack.id}`);
    },
  });

  return (
    <div className="space-y-8">
      <div className="flex flex-row items-start space-x-4">
        <Link href="/dashboard" aria-label="Back to dashboard">
          <Button variant="outline" size="icon-sm" disabled={loading}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div>
          <h2 className="text-2xl font-semibold">Create a new pack</h2>
          <p className="text-sm text-[var(--atlas-ink-muted)]">
            Import an existing GitHub repository or create a new one.
          </p>
        </div>
      </div>

      {error ? (
        <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700 space-y-2">
          <p>{error}</p>
          {errorCode === "MISSING_ATLAS_TOML" && (
            <Button
              variant="outline"
              size="sm"
              className="h-7 text-xs bg-white/50 hover:bg-white/80 border-red-200 text-red-700 hover:text-red-800"
              onClick={() => {
                setError(null);
                setErrorCode(null);
                setActiveTab("initialize");
              }}
            >
              Create a New Repository Instead
            </Button>
          )}
          {installUrl ? (
            <a href={installUrl} target="_blank" rel="noopener noreferrer">
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs bg-white/50 hover:bg-white/80 border-red-200 text-red-700 hover:text-red-800"
              >
                Install GitHub App
              </Button>
            </a>
          ) : null}
        </div>
      ) : null}

      <Tabs
        value={activeTab}
        onValueChange={setActiveTab}
        className="space-y-6"
      >
        <TabsList className="inline-flex justify-start">
          <TabsTrigger value="import">Import GitHub repo</TabsTrigger>
          <TabsTrigger value="initialize">Create GitHub repo</TabsTrigger>
        </TabsList>

        <TabsContent value="import" className="space-y-4">
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            Link a repository and Atlas will manage build and release flow.
          </p>

          <Card>
            <CardHeader>
              <CardTitle>Repository details</CardTitle>
              <CardDescription>Select a repository and set the pack name.</CardDescription>
            </CardHeader>
            <CardContent>
              <Form
                onSubmit={(event) => {
                  event.preventDefault();
                  importForm.handleSubmit();
                }}
                className="space-y-5"
              >
                <importForm.Field
                  name="owner"
                  validators={{ onChange: requireValue("Owner") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Owner</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="your-user-or-org"
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                          />
                        </FormControl>
                        <FormDescription>
                          Enter the GitHub user or organization that owns the repository.
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </importForm.Field>

                <importForm.Field
                  name="repoUrl"
                  validators={{ onChange: requireValue("Repository URL") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Repository</FormLabel>
                        <div className="flex gap-2">
                          <div className="relative flex-1">
                            <Github className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                            <Input
                              value={field.state.value}
                              readOnly
                              placeholder="Select a repository"
                              className="pl-9 cursor-pointer"
                              onClick={() => {
                                if (importForm.state.values.owner.trim()) {
                                  setRepoSelectorOpen(true);
                                }
                              }}
                            />
                          </div>
                          <Button
                            type="button"
                            variant="secondary"
                            onClick={() => {
                              if (!importForm.state.values.owner.trim()) {
                                setError("Owner is required before selecting a repository.");
                                setErrorCode("OWNER_REQUIRED");
                                return;
                              }
                              setError(null);
                              setErrorCode(null);
                              setRepoSelectorOpen(true);
                            }}
                          >
                            Choose
                          </Button>
                        </div>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </importForm.Field>

                <importForm.Field
                  name="name"
                  validators={{ onChange: requireValue("Pack name") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Pack name</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="Atlas Modpack"
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </importForm.Field>

                <importForm.Field name="slug">
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Slug (optional)</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="atlas-modpack"
                            value={field.state.value ?? ""}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </importForm.Field>

                <div className="flex flex-wrap gap-3">
                  <Button type="submit" disabled={loading}>
                    Import repository
                  </Button>
                </div>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="initialize" className="space-y-4">
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            Create a new GitHub repository with an Atlas-ready starting setup.
          </p>

          <Card>
            <CardHeader>
              <CardTitle>Repository details</CardTitle>
              <CardDescription>Pick an owner, name, and visibility.</CardDescription>
            </CardHeader>
            <CardContent>
              <Form
                onSubmit={(event) => {
                  event.preventDefault();
                  newRepoForm.handleSubmit();
                }}
                className="space-y-5"
              >
                <newRepoForm.Field
                  name="owner"
                  validators={{ onChange: requireValue("Owner") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Owner</FormLabel>
                        <FormControl>
                          <Input
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            placeholder="your-user-or-org"
                            disabled={loading}
                          />
                        </FormControl>
                        <FormDescription>
                          Enter the GitHub user or organization where Atlas App is installed.
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </newRepoForm.Field>

                <newRepoForm.Field
                  name="name"
                  validators={{ onChange: requireValue("Repository name") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Repository name</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="atlas-pack"
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    </FormField>
                  )}
                </newRepoForm.Field>

                <newRepoForm.Field name="description">
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Description (optional)</FormLabel>
                        <FormControl>
                          <Input
                            placeholder="Atlas modpack repository"
                            value={field.state.value ?? ""}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                          />
                        </FormControl>
                      </FormItem>
                    </FormField>
                  )}
                </newRepoForm.Field>

                <newRepoForm.Field name="visibility">
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Visibility</FormLabel>
                        <FormControl>
                          <select
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading}
                            className="border-input focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] h-9 w-full rounded-md border bg-transparent px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50"
                          >
                            <option value="private">Private</option>
                            <option value="public">Public</option>
                          </select>
                        </FormControl>
                      </FormItem>
                    </FormField>
                  )}
                </newRepoForm.Field>

                <Button
                  type="submit"
                  disabled={loading}
                >
                  Create repository
                </Button>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <RepositorySelector
        open={repoSelectorOpen}
        onOpenChange={setRepoSelectorOpen}
        owner={importForm.state.values.owner}
        onSelect={(repo) => {
          if (repo.owner?.login) {
            importForm.setFieldValue("owner", repo.owner.login);
          }
          importForm.setFieldValue("repoUrl", repo.html_url);
          if (!importForm.state.values.name) {
            importForm.setFieldValue("name", deriveNameFromRepoUrl(repo.html_url));
          }
          setRepoSelectorOpen(false);
        }}
        githubAppSlug={GITHUB_APP_SLUG}
      />
    </div>
  );
}
