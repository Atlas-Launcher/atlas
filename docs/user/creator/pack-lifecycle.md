---
title: Pack lifecycle
summary: Build immutability and channel promotion model for Atlas pack releases.
persona: creator
order: 3
keywords: ["lifecycle", "immutable builds", "channels", "rollback"]
intent: reference
---

# Pack lifecycle

This page explains the release model that Atlas uses across creator workflows.

## Source in Git

Your repository is the source of truth for pack definitions and config.

- Commit content changes to Git.
- Trigger builds from committed source.

## Builds are immutable

Each build artifact is fixed after creation.

- You do not edit an existing build in place.
- Rebuild from source for content changes.

## Channels are mutable pointers

Channels map environments to specific build IDs.

- Typical flow: `dev` -> `beta` -> `production`.
- Promotion changes channel pointers, not build contents.

## Consumer behavior

Launcher and runner resolve content through channel pointers.

- Players receive whichever build the chosen channel currently points to.
- Server hosts receive whichever build their configured channel points to.

## Rollback approach

Rollback is a channel operation, not a rebuild operation.

- Repoint the affected channel to a known-good build.
- Verify behavior, then continue forward promotion.
