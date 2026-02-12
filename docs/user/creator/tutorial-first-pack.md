---
title: Creator First Pack Tutorial
summary: Step-by-step walkthrough from login and initialization through publish and promotion.
persona: creator
order: 2
keywords: ["tutorial", "first pack", "publish", "promote"]
intent: tutorial
---

# Tutorial: First Pack Release (Creator)

This walkthrough takes you from repo setup to a production-ready channel promotion.

## Step 1: Sign in

```bash
atlas login
atlas status
```

Expected result:
- `atlas status` shows an active session.

## Step 2: Initialize or refresh pack config

New repo:

```bash
atlas init
```

Existing repo needing version refresh:

```bash
atlas reinit
```

Optional validation before building:

```bash
atlas validate
```

## Step 3: Build

```bash
atlas build
```

Expected result:
- A local build artifact is generated.

## Step 4: Publish

```bash
atlas publish
```

Expected result:
- Build is uploaded and registered on your configured channel.

## Step 5: Promote

When testing is complete, promote the selected build:

```bash
atlas promote
```

Interactive mode lets you choose pack/channel/build if not provided.

## Step 6: Commit and push source changes

```bash
atlas commit "Update pack content"
atlas push
```

## Optional: CI workflow bootstrap

```bash
atlas workflow init
```

Use this once per repo to sync Atlas CI workflow scaffolding.

## Troubleshooting

- Not signed in: run `atlas login`.
- Non-interactive errors: pass explicit IDs/flags (for example `--pack-id`, `--build-id`, `--channel`).
- Push/pull auth issues: `atlas pull` and `atlas push` now use your system Git credentials.
