---
title: Server host getting started
summary: First-time Atlas Runner setup and initial server operation flow.
persona: host
order: 1
keywords: ["atlas-runner", "server start", "daemon", "install"]
intent: getting-started
---

# Server host getting started

Use this guide once to install Atlas Runner and launch your first server.

## Choose the host platform

Linux VPS is the recommended target for always-on server workloads.

- Linux VPS: recommended for production hosting.
- macOS: supported for local or small-scale usage.
- WSL: supported, usually with daemon install skipped.

## Step 1: Install Atlas Runner

Install the runner binary first.

Linux install:

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s --
```

WSL install without daemon registration:

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s -- --no-daemon-install
```

Verify installation:

```bash
atlas-runner --version
```

## Step 2: Link the host to Atlas

Authenticate and select the pack/channel association.

```bash
atlas-runner auth login
```

## Step 3: Start the server

Use the start command to initialize and launch the server runtime.

```bash
atlas-runner server start
```

## Step 4: Run core operations

Use these commands for immediate day-one operation.

```bash
atlas-runner server logs --follow
atlas-runner server console
atlas-runner server stop
```

## Step 5: Check daemon health

Verify daemon state when logs or commands look stale.

```bash
atlas-runner daemon status
atlas-runner daemon logs --follow
```
