# Atlas CLI (Pack Tooling) Developer Guide

Project: `apps/cli`

Binary name: `atlas`

## Responsibilities

- Pack authoring and validation workflows.
- Build artifact generation from source config.
- CI workflow sync/bootstrap.
- Deploy/build completion path against Hub APIs.

## Command Groups (Current)

Defined in `apps/cli/src/main.rs`:
- `pack` (init/reinit/channel/build/add/rm/list/pull/push/commit/validate)
- `auth`
- `ci` (init/update)
- `deploy`
- `completion`

## Current Usage Intent

- Creator-facing pack workflow and deployment automation.
- Integrates with web API for build upload/presign/complete flow.

## Platform Notes

No standalone support matrix is published for this CLI in current docs policy.
Treat platform behavior as best-effort developer tooling unless explicitly documented.

## Query Normalization Notes

- MRPack override filename query extraction strips numeric-only tokens and Minecraft version markers (for example `mc1.20.1` -> no `mc1` token).
- This keeps search queries focused on mod identity terms such as `create`, `sodium`, `fabric`, and avoids noisy version fragments.

## Warning Hygiene

- Keep CLI command modules warning-clean under `cargo check --workspace`.
- Remove stale request/response structs and helper functions once command flows migrate to `atlas_client::hub` APIs.
