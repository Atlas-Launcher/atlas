---
title: Creator getting started
summary: Core Atlas CLI workflow to build, publish, and promote pack releases.
persona: creator
order: 1
keywords: ["atlas init", "atlas build", "atlas publish", "atlas promote"]
intent: getting-started
---

# Creator getting started

Use this guide to establish the standard release workflow for your pack.

## Before you begin

Prepare tooling and access before running pack commands.

- A Git repository for your pack source.
- Atlas CLI installed (`atlas --version`).
- Atlas account with access to the target pack in Hub.

## Core release workflow

Use these commands in order for a standard release cycle.

1. Initialize pack config: `atlas init`
2. Build an artifact: `atlas build`
3. Publish to Hub: `atlas publish`
4. Promote to the next channel: `atlas promote`

## High-use command groups

These command groups cover the majority of creator operations.

- Auth: `atlas login`, `atlas logout`, `atlas status`
- Source setup: `atlas init`, `atlas reinit`, `atlas validate`, `atlas commit`
- Source sync: `atlas pull`, `atlas push`
- Release: `atlas build`, `atlas publish`, `atlas promote`
- Mod pointers: `atlas mod add`, `atlas mod remove`, `atlas mod list`, `atlas mod import`
- CI setup: `atlas workflow init`, `atlas workflow update`

## Recommended channel policy

Promote forward through channels instead of rebuilding the same commit.

1. Publish to `dev`.
2. Validate in test environments.
3. Promote to `beta`.
4. Promote to `production` after signoff.

## Next step

Run `tutorial-first-pack.md` to execute one full release with expected outcomes.
