---
name: atlas-context-sources
description: Find and prioritize authoritative context sources for Atlas monorepo work. Use when requests involve Atlas architecture, launcher/runner/web behavior, protocol and data formats, API routes, authentication, release flow, or when Codex needs to answer "where should I look first?" before implementing, debugging, reviewing, or documenting changes.
---

# Atlas Context Sources

Use this skill to avoid shallow context gathering. Start with executable truth, then layer architecture and external docs.

## Source Priority

1. Executable truth in this repository.
2. Atlas design and developer docs.
3. Git history for behavior changes.
4. Official external docs matching pinned versions.
5. Community posts only when primary sources are insufficient.

Prefer code over prose when they disagree. Treat docs as intent, not ground truth.

## Workflow

1. Classify the request domain using [references/domain-map.md](./references/domain-map.md).
2. Open at least one executable source and one supporting doc from that domain.
3. Check pinned versions (`package.json`, crate `Cargo.toml`) before reading external docs.
4. Reconcile inconsistencies and state what source won and why.
5. Summarize with exact file paths, unresolved questions, and confidence.

## Minimum Evidence Bar

- Bug fix: failing path in code + architecture/context doc + validation command or test.
- Feature work: entrypoint + contract/schema + release/deployment touchpoint.
- API changes: route handler + shared types/schema + at least one caller.
- Data format changes: proto or migration + serializer/parser + user/dev doc impact.

## Fast Lookup Commands

```bash
rg -n "keyword" apps crates docs
rg --files docs/dev docs/user
rg -n "route.ts|GET\\(|POST\\(" apps/web/app/api
rg -n "proto|manifest|pack|channel|build" crates apps
```

## References

- Atlas domain routing and best local files: [references/domain-map.md](./references/domain-map.md)
- External primary docs by stack: [references/external-sources.md](./references/external-sources.md)
