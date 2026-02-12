# Runner-v2 CLI Developer Guide

Project: `apps/runner-v2`

Binary name: `atlas-runner`

## Responsibilities

- User-facing CLI for runner daemon operations.
- Auth/bootstrap commands.
- Start/stop/log/command control over local daemon via IPC.
- Optional Linux systemd installer path for daemon service.
  - `host install` also checks the latest Distribution API `runner` release and replaces `/usr/local/bin/atlas-runner` when the running CLI version is older.
  - If `--runnerd-path` is not passed, `host install` resolves latest `runnerd` via Distribution API v1 and downloads through `/api/v1/download/{download_id}`.
  - Generated unit includes `ATLAS_SYSTEMD_MANAGED=1` for runnerd systemd-mode features (including daemon-side self-update eligibility checks).
  - Re-running `host install` reconciles managed keys in `atlas-runnerd.service` while preserving unknown/custom directives.

## Command Surface (Current)

Defined in `apps/runner-v2/src/main.rs`.

Command groups:
- `auth login`
- `server start`
- `server stop`
- `server logs`
- `server command`
- `server console`
- `server backup`
- `daemon status`
- `daemon stop`
- `daemon logs`
- `host install` (Linux-only)
- `host path`

`auth login --hub-url` resolution order:
- CLI flag `--hub-url`
- env `ATLAS_HUB_URL`
- existing `deploy.json` `hub_url`
- default `https://atlas.nathanm.org`

`atlas-runner` build-time version resolution:
- compile-time env `ATLAS_BUILD_VERSION` (tag-derived in release CI)
- fallback to Cargo package version for local/non-tag builds

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
- `host install` systemd install command path.
