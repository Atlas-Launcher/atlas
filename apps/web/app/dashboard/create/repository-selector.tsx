"use client";

import { useCallback, useEffect, useState } from "react";
import { Search, ChevronLeft, ChevronRight, Loader2, Book } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";

function useDebounce<T>(value: T, delay: number): T {
    const [debouncedValue, setDebouncedValue] = useState(value);

    useEffect(() => {
        const handler = setTimeout(() => {
            setDebouncedValue(value);
        }, delay);

        return () => {
            clearTimeout(handler);
        };
    }, [value, delay]);

    return debouncedValue;
}

type GithubRepo = {
    name: string;
    full_name: string;
    html_url: string;
    clone_url: string;
    owner?: {
        login?: string;
    };
};

type RepositorySelectorProps = {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    owner: string;
    onSelect: (repo: GithubRepo) => void;
    githubAppSlug?: string;
};

export function RepositorySelector({
    open,
    onOpenChange,
    owner,
    onSelect,
    githubAppSlug,
}: RepositorySelectorProps) {
    const [repos, setRepos] = useState<GithubRepo[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [page, setPage] = useState(1);
    const [hasNextPage, setHasNextPage] = useState(false);
    const [search, setSearch] = useState("");
    const debouncedSearch = useDebounce(search, 500);

    const [installUrl, setInstallUrl] = useState<string | null>(null);

    const loadRepos = useCallback(async (pageToLoad: number, searchQuery: string) => {
        if (!owner.trim()) {
            setRepos([]);
            setHasNextPage(false);
            setPage(1);
            setError("Enter an owner before loading repositories.");
            return;
        }

        setLoading(true);
        setError(null);
        setInstallUrl(null);
        try {
            const params = new URLSearchParams({
                owner: owner.trim(),
                page: pageToLoad.toString(),
                per_page: "6", // 6 items per page fits nicely
            });
            if (searchQuery) {
                params.set("search", searchQuery);
            }

            const response = await fetch(`/api/v1/github/repos?${params.toString()}`);
            const data = await response.json();

            if (!response.ok) {
                // Check for installation URL in the error response
                if (data.installUrl) {
                    setInstallUrl(data.installUrl);
                }
                throw new Error(data.error || "Failed to load repositories");
            }

            setRepos(data.repos || []);
            setHasNextPage(!!data.nextPage);
            setPage(pageToLoad);
        } catch (err) {
            const message = err instanceof Error ? err.message : "Unable to load repositories. Please try again.";
            setError(message);
        } finally {
            setLoading(false);
        }
    }, [owner]);

    useEffect(() => {
        if (open) {
            loadRepos(1, debouncedSearch);
        }
    }, [open, debouncedSearch, loadRepos]);

    const handleNextPage = () => {
        if (hasNextPage && !loading) {
            loadRepos(page + 1, debouncedSearch);
        }
    };

    const handlePrevPage = () => {
        if (page > 1 && !loading) {
            loadRepos(page - 1, debouncedSearch);
        }
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-2xl">
                <DialogHeader>
                    <DialogTitle>Select a repository</DialogTitle>
                </DialogHeader>

                <div className="space-y-4">
                    <p className="text-xs text-[var(--atlas-ink-muted)]">
                        Owner: <span className="font-medium">{owner || "not set"}</span>
                    </p>
                    <div className="relative">
                        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--atlas-ink-muted)]" />
                        <Input
                            placeholder="Search repositories"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="pl-9"
                            disabled={!owner.trim()}
                        />
                    </div>

                    <div className="min-h-[300px]">
                        {loading ? (
                            <div className="flex h-[300px] items-center justify-center">
                                <Loader2 className="h-8 w-8 animate-spin text-[var(--atlas-ink-muted)]" />
                            </div>
                        ) : error ? (
                            <div className="flex h-[300px] flex-col items-center justify-center gap-3 text-center px-4">
                                <p className="text-sm text-red-500">{error}</p>
                                <div className="flex gap-2">
                                    {installUrl && (
                                        <a
                                            href={installUrl}
                                            target="_blank"
                                            rel="noopener noreferrer"
                                        >
                                            <Button size="sm">
                                    Install GitHub app
                                            </Button>
                                        </a>
                                    )}
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => loadRepos(page, debouncedSearch)}
                                    >
                                        Try Again
                                    </Button>
                                </div>
                            </div>
                        ) : repos.length === 0 ? (
                            <div className="flex h-[300px] flex-col items-center justify-center gap-2 text-center">
                                <p className="text-sm text-[var(--atlas-ink-muted)]">
                                    {search
                                        ? "No repositories found matching your search."
                                        : "No repositories found."}
                                </p>
                                {!search && githubAppSlug && (
                                    <a
                                        href={`https://github.com/apps/${githubAppSlug}/installations/new`}
                                        target="_blank"
                                        rel="noopener noreferrer"
                                    >
                                        <Button variant="outline" size="sm">
                                            Configure GitHub app
                                        </Button>
                                    </a>
                                )}
                            </div>
                        ) : (
                            <div className="grid gap-3 sm:grid-cols-2">
                                {repos.map((repo) => (
                                    <button
                                        key={repo.html_url}
                                        onClick={() => onSelect(repo)}
                                        className="flex flex-col items-start rounded-xl border border-[hsl(var(--border)/0.8)] bg-[var(--atlas-surface-soft)] p-4 text-left transition hover:border-[hsl(var(--primary)/0.5)] hover:bg-[var(--atlas-surface-strong)] hover:shadow-sm focus:outline-none focus:ring-2 focus:ring-[hsl(var(--ring))]"
                                    >
                                        <div className="mb-2 flex items-center gap-2">
                                            {repo.owner ? (
                                                <div className="flex h-6 w-6 items-center justify-center rounded-md bg-[var(--atlas-surface-strong)] font-mono text-xs font-bold text-[var(--atlas-ink-muted)]">
                                                    {repo.owner.login?.slice(0, 1).toUpperCase()}
                                                </div>
                                            ) : (
                                                <Book className="h-4 w-4 text-[var(--atlas-ink-muted)]" />
                                            )}
                                            <span className="truncate text-sm font-medium text-[var(--atlas-ink)]">
                                                {repo.name}
                                            </span>
                                        </div>
                                        <p className="truncate text-xs text-[var(--atlas-ink-muted)]">{repo.full_name}</p>
                                    </button>
                                ))}
                            </div>
                        )}
                    </div>

                    <div className="flex items-center justify-between border-t pt-4">
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={handlePrevPage}
                            disabled={page <= 1 || loading}
                        >
                            <ChevronLeft className="mr-2 h-4 w-4" />
                            Previous
                        </Button>
                        <span className="text-xs text-[var(--atlas-ink-muted)]">Page {page}</span>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={handleNextPage}
                            disabled={!hasNextPage || loading}
                        >
                            Next
                            <ChevronRight className="ml-2 h-4 w-4" />
                        </Button>
                    </div>

                    {githubAppSlug && (
                        <div className="text-center text-xs pt-2">
                            <a
                                href={`https://github.com/apps/${githubAppSlug}/installations/new`}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-[var(--atlas-ink-muted)] hover:text-[hsl(var(--primary))] hover:underline transition-colors"
                            >
                                Don&apos;t see your repository? Configure access.
                            </a>
                        </div>
                    )}
                </div>
            </DialogContent>
        </Dialog>
    );
}
