"use client";

import { useEffect, useMemo, useState } from "react";
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
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

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

type GithubOwner = {
  login: string;
  type: "user" | "org";
  avatarUrl?: string | null;
};

const requireValue =
  (label: string) =>
  ({ value }: { value: string }) =>
    value?.trim() ? undefined : `${label} is required.`;

export default function CreatePackClient() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [githubOwners, setGithubOwners] = useState<GithubOwner[]>([]);
  const [githubLoading, setGithubLoading] = useState(false);
  const [githubError, setGithubError] = useState<string | null>(null);

  const createPack = async (payload: {
    name: string;
    slug?: string;
    repoUrl?: string;
  }) => {
    const response = await fetch("/api/packs", {
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
    const result = await createPack(payload);
    setLoading(false);

    if (!result.ok) {
      setError(result.data?.error ?? "Unable to create pack.");
      return;
    }

    router.push(`/dashboard/${result.data.pack.id}`);
  };

  const importForm = useForm({
    defaultValues: {
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

      const response = await fetch("/api/github/repos", {
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
        setLoading(false);
        return;
      }

      const repoName = data?.repo?.name ?? value.name;
      const repoUrl = data?.repo?.htmlUrl ?? data?.repo?.cloneUrl ?? "";
      const packResult = await createPack({
        name: repoName,
        repoUrl,
      });

      setLoading(false);

      if (!packResult.ok) {
        setError(packResult.data?.error ?? "Repository created, but pack failed.");
        return;
      }

      router.push(`/dashboard/${packResult.data.pack.id}`);
    },
  });

  useEffect(() => {
    let mounted = true;
    const loadOwners = async () => {
      setGithubLoading(true);
      setGithubError(null);
      const response = await fetch("/api/github/owners");
      const data = await response.json();
      if (!mounted) {
        return;
      }
      setGithubLoading(false);
      if (!response.ok) {
        setGithubError(data?.error ?? "Unable to load GitHub owners.");
        return;
      }
      setGithubOwners(data.owners ?? []);
    };

    loadOwners().catch(() => {
      if (mounted) {
        setGithubLoading(false);
        setGithubError("Unable to load GitHub owners.");
      }
    });

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    if (githubOwners.length && !newRepoForm.state.values.owner) {
      newRepoForm.setFieldValue("owner", githubOwners[0].login);
    }
  }, [githubOwners, newRepoForm]);

  const ownerOptions = useMemo(
    () =>
      githubOwners.map((owner) => ({
        label: `${owner.login}${owner.type === "org" ? " (org)" : ""}`,
        value: owner.login,
      })),
    [githubOwners]
  );

  return (
    <div className="space-y-8">
      <div className="flex flex-row items-start space-x-4">
        <Link href="/dashboard" aria-label="Back to dashboard">
          <Button variant="outline" size="icon-sm" disabled={loading}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
        </Link>
        <div>
          <h2 className="text-2xl font-semibold">Let&apos;s build something new</h2>
          <p className="text-sm text-[var(--atlas-ink-muted)]">
            Import a GitHub repository or create a new one.
          </p>
        </div>
      </div>

      {error ? (
        <div className="rounded-2xl border border-red-200 bg-red-50 px-4 py-3 text-xs text-red-700">
          {error}
        </div>
      ) : null}

      <Tabs defaultValue="import" className="space-y-6">
        <TabsList className="inline-flex justify-start">
          <TabsTrigger value="import">Import From GitHub Repository</TabsTrigger>
          <TabsTrigger value="initialize">New GitHub Repository</TabsTrigger>
        </TabsList>

        <TabsContent value="import" className="space-y-4">
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            Link a repo and Atlas will keep builds flowing into your channels.
          </p>

          <Card>
            <CardHeader>
              <CardTitle>Repository details</CardTitle>
              <CardDescription>Paste a repo URL and set the pack name.</CardDescription>
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
                  name="repoUrl"
                  validators={{ onChange: requireValue("Repository URL") }}
                >
                  {(field) => (
                    <FormField field={field}>
                      <FormItem>
                        <FormLabel>Repository URL</FormLabel>
                        <InputGroup>
                          <InputGroupAddon>
                            <InputGroupText>
                              <Github className="h-4 w-4" />
                            </InputGroupText>
                          </InputGroupAddon>
                          <FormControl>
                            <InputGroupInput
                              placeholder="https://github.com/org/repo"
                              value={field.state.value}
                              onChange={(event) => field.handleChange(event.target.value)}
                              onBlur={() => {
                                field.handleBlur();
                                const derived = deriveNameFromRepoUrl(field.state.value);
                                if (derived && !importForm.state.values.name.trim()) {
                                  importForm.setFieldValue("name", derived);
                                }
                              }}
                              disabled={loading}
                            />
                          </FormControl>
                        </InputGroup>
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
                    Import
                  </Button>
                </div>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="initialize" className="space-y-4">
          <p className="text-xs text-[var(--atlas-ink-muted)]">
            Create a brand new GitHub repository with a default Atlas-ready setup.
          </p>

          {githubError ? (
            <div className="rounded-2xl border border-amber-200 bg-amber-50 px-4 py-3 text-xs text-amber-700">
              {githubError}
            </div>
          ) : null}

          <Card>
            <CardHeader>
              <CardTitle>Repository details</CardTitle>
              <CardDescription>Pick an owner and configure the repo.</CardDescription>
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
                          <select
                            value={field.state.value}
                            onChange={(event) => field.handleChange(event.target.value)}
                            onBlur={field.handleBlur}
                            disabled={loading || githubLoading || ownerOptions.length === 0}
                            className="border-input focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] h-9 w-full rounded-md border bg-transparent px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none disabled:cursor-not-allowed disabled:opacity-50"
                          >
                            {ownerOptions.length === 0 ? (
                              <option value="">No linked GitHub accounts</option>
                            ) : null}
                            {ownerOptions.map((option) => (
                              <option key={option.value} value={option.value}>
                                {option.label}
                              </option>
                            ))}
                          </select>
                        </FormControl>
                        <FormDescription>
                          {githubLoading
                            ? "Loading GitHub owners..."
                            : "Choose from the GitHub accounts linked to your profile."}
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
                  disabled={loading || githubLoading || ownerOptions.length === 0}
                >
                  Create Pack Repository
                </Button>
              </Form>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
