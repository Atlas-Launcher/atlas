---
title: Atlas User Docs
summary: Persona-first documentation index for players, creators, and server hosts.
persona: creator
order: 90
keywords: ["docs", "index", "personas"]
intent: reference
---

# Atlas User Docs

These docs are organized by persona, with quick-start guides first and cheat sheets for day-to-day use.

## Choose your path

- Player: install Atlas Launcher and join packs.
- Creator: build, publish, and promote pack releases.
- Server host: run Atlas Runner on Linux/macOS/WSL for dedicated servers.

## Player

- `player/README.md`
- `player/getting-started.md`
- `player/quick-reference.md`
- `player/troubleshooting.md`

## Creator

- `creator/README.md`
- `creator/getting-started.md`
- `creator/tutorial-first-pack.md`
- `creator/pack-lifecycle.md`
- `creator/quick-reference.md`

## Server Host

- `host/README.md`
- `host/getting-started.md`
- `host/quick-reference.md`
- `host/troubleshooting.md`

## Shared

- `runner-install.md`
- `user-stories.md`

## Frontmatter authoring rules

Every markdown file in `docs/user` must include frontmatter with this schema:

- `title: string`
- `summary: string`
- `persona: "player" | "creator" | "host"`
- `order: number`
- `keywords: string[]`
- `intent: "getting-started" | "reference" | "troubleshooting" | "tutorial"`

Use this format:

```md
---
title: Example
summary: One sentence overview.
persona: player
order: 1
keywords: ["keyword-one", "keyword-two"]
intent: reference
---
```

If you add or reorder public user docs, update `navigation.json` to keep sidebar and previous/next links deterministic.
