---
title: Server Host Getting Started
summary: First-time Atlas Runner setup and operating flow for Linux VPS, macOS, and WSL.
persona: host
order: 1
keywords: ["atlas-runner", "server start", "daemon", "install"]
intent: getting-started
---

# Server Host Getting Started

This guide covers first-time setup for Atlas Runner on Linux VPS, macOS, or WSL.

## Recommended platform

Use Linux VPS for always-on servers.

## Step 1: Install runner tooling

Linux one-command install:

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s --
```

WSL (skip daemon install):

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s -- --no-daemon-install
```

Verify:

```bash
atlas-runner --version
```

## Step 2: Link runner to your Atlas pack

```bash
atlas-runner auth login
```

Interactive mode will guide you through pack selection and token naming.

## Step 3: Start server

```bash
atlas-runner server start
```

What this handles:
- build retrieval by channel,
- first-run setup prompts,
- server process launch through runnerd.

## Step 4: Operate server

Check server logs:

```bash
atlas-runner server logs --follow
```

Open interactive console:

```bash
atlas-runner server console
```

Stop server:

```bash
atlas-runner server stop
```

## Step 5: Check daemon health

```bash
atlas-runner daemon status
atlas-runner daemon logs --follow
```

## Notes

- `atlas-runner host install` manages systemd setup on Linux.
- Keep `atlas-runner` and `atlas-runnerd` on matching versions.
