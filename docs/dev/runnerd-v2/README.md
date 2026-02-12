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

## Platform Notes

Supported:
- macOS
- Linux
- WSL
