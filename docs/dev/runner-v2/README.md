# Runner-v2 CLI Developer Guide

Project: `apps/runner-v2`

Binary name: `atlas-runner`

## Responsibilities

- User-facing CLI for runner daemon operations.
- Auth/bootstrap commands.
- Start/stop/log/exec control over local daemon via IPC.
- Optional Linux systemd installer path for daemon service.
  - If `--runnerd-path` is not passed, `install` resolves latest `runnerd` via Distribution API v1 and downloads through `/api/v1/download/{download_id}`.

## Command Surface (Current)

Defined in `apps/runner-v2/src/main.rs`.

Top-level commands:
- `auth`
- `ping`
- `shutdown`
- `down`
- `up`
- `exec`
- `logs`
- `cd`
- `install` (Linux-only)
- `backup`

`auth --hub-url` resolution order:
- CLI flag `--hub-url`
- env `ATLAS_HUB_URL`
- existing `deploy.json` `hub_url`
- default `https://atlas.nathanm.org`

## Runtime Model

- CLI connects to daemon over local socket (runner IPC v2).
- If daemon is missing, CLI attempts to start it.
- Daemon remains source of truth for server process lifecycle.

## Platform Notes

Supported:
- macOS
- Linux
- WSL

Linux-specific capability:
- systemd install command path.
