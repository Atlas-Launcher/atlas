---
title: Atlas Runner Install
summary: VPS-first install guide for Atlas Runner and atlas-runnerd service setup.
persona: host
order: 4
keywords: ["runner install", "vps", "systemd", "linux"]
intent: getting-started
---

# Atlas Runner Install (VPS-first)

This guide assumes you are installing on a Linux VPS or dedicated host.

For full day-two operations, see:
- `host/getting-started.md`
- `host/quick-reference.md`
- `host/troubleshooting.md`

## Quick Install

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s --
```

What this does:
- Detects Linux architecture (`x64` or `arm64`)
- Downloads the latest stable runner build
- Installs to `/usr/local/bin/atlas-runner`
- Runs `atlas-runner host install` to install/update `atlas-runnerd` systemd service

WSL install (skip daemon install):

```bash
curl -fsSL "${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/install" | sudo bash -s -- --no-daemon-install
```

## Verify

```bash
atlas-runner --version
```

## First Run

```bash
atlas-runner auth login
atlas-runner server start
```

## Manual Download Links

- `${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/latest/linux/x64`
- `${NEXT_PUBLIC_BETTER_AUTH_URL%/}/download/runner/latest/linux/arm64`

## Notes

- Linux is the recommended install path for servers and VPS environments.
- macOS is natively supported, but you must manually download both `atlas-runner` and `atlas-runnerd`.
- Windows hosts should run Atlas Runner through WSL and use `--no-daemon-install`.
- Runner daemon setup now runs during install by default unless explicitly skipped.
- When installed as a systemd service with `ATLAS_SYSTEMD_MANAGED=1` and running as root, runnerd can stage and apply stable `runner`/`runnerd` binary updates during the midnight backup window.
