---
title: Creator Getting Started
summary: Fastest path to initialize, build, publish, and promote your first Atlas pack release.
persona: creator
order: 1
keywords: ["atlas init", "atlas build", "atlas publish", "atlas promote"]
intent: getting-started
---

# Creator Getting Started

This guide covers the fastest path to ship your first Atlas pack build.

## What you need

- A Git repository for your pack.
- Atlas CLI installed (`atlas --version`).
- Atlas account access to your pack in Hub.

## Core workflow

1. Initialize pack config:

```bash
atlas init
```

2. Build from source:

```bash
atlas build
```

3. Publish the build to Hub:

```bash
atlas publish
```

4. Promote a build to another channel when ready:

```bash
atlas promote
```

## Day-to-day command groups

- Auth: `atlas login`, `atlas logout`, `atlas status`
- Pack setup: `atlas init`, `atlas reinit`, `atlas validate`, `atlas commit`
- Source sync: `atlas pull`, `atlas push`
- Releases: `atlas build`, `atlas publish`, `atlas promote`
- Mod pointers: `atlas mod add|remove|list|import`
- CI workflow: `atlas workflow init|update`

## Recommended release sequence

1. Publish to `dev`.
2. Validate in testing.
3. Promote to `beta`.
4. Promote to `production` once stable.

## Next docs

- Hands-on tutorial: `tutorial-first-pack.md`
- Lifecycle model: `pack-lifecycle.md`
- Command cheat sheet: `quick-reference.md`
