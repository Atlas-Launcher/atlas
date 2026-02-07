"use client";

import { useEffect, useState } from "react";
import { Search, ChevronLeft, ChevronRight, Loader2, GitFork, Book } from "lucide-react";
import { useDebounce } from "use-debounce";

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";

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
    onSelect: (repo: GithubRepo) => void;
    githubAppSlug?: string;
};

export function RepositorySelector({
    open,
    onOpenChange,
    onSelect,
    githubAppSlug,
}: RepositorySelectorProps) {
    const [repos, setRepos] = useState<GithubRepo[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [page, setPage] = useState(1);
    const [hasNextPage, setHasNextPage] = useState(false);
    const [search, setSearch] = useState("");
    const [debouncedSearch] = useDebounce(search, 500);

    useEffect(() => {
        if (open) {
            loadRepos(1, debouncedSearch);
        }
    }, [open, debouncedSearch]);

    const loadRepos = async (pageToLoad: number, searchQuery: string) => {
        setLoading(true);
        setError(null);
        try {
            const params = new URLSearchParams({
                page: pageToLoad.toString(),
                per_page: "6", // 6 items per page fits nicely
            });
            if (searchQuery) {
                params.set("search", searchQuery);
            }

            const response = await fetch(`/api/github/repos?${params.toString()}`);
            if (!response.ok) {
                throw new Error("Failed to load repositories");
            }

            const data = await response.json();
            setRepos(data.repos || []);
            setHasNextPage(!!data.nextPage);
            setPage(pageToLoad);
        } catch (err) {
            setError("Unable to load repositories. Please try again.");
        } finally {
            setLoading(false);
        }
    };

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
                    <DialogTitle>Select a Repository</DialogTitle>
                </DialogHeader>

                <div className="space-y-4">
                    <div className="relative">
                        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
                        <Input
                            placeholder="Search repositories..."
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="pl-9"
                        />
                    </div>

                    <div className="min-h-[300px]">
                        {loading ? (
                            <div className="flex h-[300px] items-center justify-center">
                                <Loader2 className="h-8 w-8 animate-spin text-gray-400" />
                            </div>
                        ) : error ? (
                            <div className="flex h-[300px] flex-col items-center justify-center gap-2 text-center">
                                <p className="text-sm text-red-500">{error}</p>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => loadRepos(page, debouncedSearch)}
                                >
                                    Try Again
                                </Button>
                            </div>
                        ) : repos.length === 0 ? (
                            <div className="flex h-[300px] flex-col items-center justify-center gap-2 text-center">
                                <p className="text-sm text-gray-500">
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
                                            Configure GitHub App
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
                                        className="flex flex-col items-start rounded-xl border border-gray-200 p-4 text-left transition hover:border-blue-500 hover:bg-blue-50/50 hover:shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                                    >
                                        <div className="mb-2 flex items-center gap-2">
                                            {repo.owner ? (
                                                <div className="flex h-6 w-6 items-center justify-center rounded-md bg-gray-100 font-mono text-xs font-bold text-gray-600">
                                                    {repo.owner.login?.slice(0, 1).toUpperCase()}
                                                </div>
                                            ) : (
                                                <Book className="h-4 w-4 text-gray-400" />
                                            )}
                                            <span className="truncate text-sm font-medium text-gray-900">
                                                {repo.name}
                                            </span>
                                        </div>
                                        <p className="truncate text-xs text-gray-500">{repo.full_name}</p>
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
                        <span className="text-xs text-gray-500">Page {page}</span>
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
                </div>
            </DialogContent>
        </Dialog>
    );
}
