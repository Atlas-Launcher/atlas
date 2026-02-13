---
title: Creator first pack tutorial
summary: End-to-end command walkthrough from login to production promotion.
persona: creator
order: 2
keywords: ["tutorial", "first pack", "publish", "promote"]
intent: tutorial
---

# Creator first pack tutorial

Use this walkthrough to run one complete release with clear checkpoints.

## Step 1: Sign in

Start by confirming your CLI session is valid.

```bash
atlas login
atlas status
```

Expected result: `atlas status` shows an active session.

## Step 2: Prepare pack configuration

Initialize new repos or refresh existing pack metadata.

```bash
atlas init
```

If the repo is already initialized and needs refresh:

```bash
atlas reinit
```

Optional validation before building:

```bash
atlas validate
```

## Step 3: Build the artifact

Create a release artifact from the current source state.

```bash
atlas build
```

Expected result: the build command completes without validation errors.

## Step 4: Publish to Hub

Upload and register the build for channel use.

```bash
atlas publish
```

Expected result: the build is available on the target channel.

## Step 5: Promote when testing passes

Move a validated build to the next channel.

```bash
atlas promote
```

Expected result: the selected channel points to the promoted build.

## Step 6: Commit and push source changes

Store release-related source edits in Git after verification.

```bash
atlas commit "Release update"
atlas push
```

## Troubleshooting notes

Use these checks if a step fails in this tutorial flow.

- Authentication failure: run `atlas login`.
- Non-interactive execution: pass explicit IDs and channel flags.
- Git sync error: run `atlas pull` before retrying `atlas push`.
