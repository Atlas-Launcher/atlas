# Runnerd-v2 Developer Guide

Project: `apps/runnerd-v2`

Binary name: `atlas-runnerd`

## Responsibilities

- Long-running server daemon.
- Applies pack blobs into runtime server directories.
- Supervises Minecraft process lifecycle.
- Handles logs, updates, backup operations, and control-plane events.

## Architecture

Entrypoint: `apps/runnerd-v2/src/main.rs`

Major subsystems:
- `daemon`: IPC request handling loop.
- `supervisor`: start/stop/apply/log/update orchestration.
- `config`: runtime/deploy configuration persistence.
- `backup`: backup operations.

## Runtime Behavior Highlights

- Single-instance lock + stale socket handling.
- Detects existing Minecraft process and exits to avoid conflict.
- Applies launch plan with Java memory flags and normalization behavior.
- Uses `runner-provision-v2` for apply/provision logic.
- Update/whitelist watcher loops use `deploy.json` `hub_url` for runner token exchange and polling.
- Self-update subsystem (Linux only) checks Distribution API stable releases for `runner` + `runnerd` at daemon startup and every 6 hours.
- Self-update is active only when `ATLAS_SYSTEMD_MANAGED=1` and runnerd runs as root (uid 0).
- Staged updates are applied after the daily midnight backup pass, with managed-key reconciliation for `/etc/systemd/system/atlas-runnerd.service` followed by `systemctl restart atlas-runnerd.service`.

`atlas-runnerd` build-time version resolution:
- compile-time env `ATLAS_BUILD_VERSION` (tag-derived in release CI)
- fallback to Cargo package version for local/non-tag builds

## Platform Notes

Supported:
- macOS
- Linux
- WSL

## Troubleshooting

If you see `HTTP status client error (401 Unauthorized)` for
`/api/v1/runner/exchange`, runnerd cannot validate the configured service token.
Common causes are:

- The token in `deploy.json` is not a runner service token.
- The token was revoked or expired.
- The token was created on a different Hub environment than `hub_url`.

Runner service tokens are created via `/api/v1/runner/tokens` and currently
start with `atlas_runner_`.
