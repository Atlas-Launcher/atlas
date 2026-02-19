# Atlas CLI (Pack Tooling) Developer Guide

Project: `apps/cli`

Binary name: `atlas`

## Responsibilities

- Pack authoring and validation workflows.
- Build artifact generation from source config.
- CI workflow sync/bootstrap.
- Deploy/build completion path against Hub APIs.

## Command Surface (Current)

Defined in `apps/cli/src/main.rs`:
- `login`
- `logout`
- `status`
- `init`
- `reinit`
- `pull`
- `push`
- `build`
- `publish`
- `promote`
- `validate`
- `commit`
- `mod` (`add`, `remove`, `list`, `import`)
- `workflow` (`init`, `update`)
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

## Runtime Notes

- `atlas_client::hub` blocking helper methods now bootstrap their own Tokio runtime when no runtime exists (normal synchronous CLI execution, including GitHub Actions shell steps).
- This prevents `there is no reactor running` panics when `atlas` commands call Hub APIs from non-async contexts.
- `atlas mod add` now resolves compatible versions before writing pointers, and always pins an explicit version selector.
- When multiple compatible versions are available, interactive runs present a version picker with newest preselected by default.
- `atlas mod add` auto-installs dependency mods by default and writes compatibility metadata into pointer TOMLs (`compat.minecraft`, `compat.loaders`, `compat.loader_versions`, `compat.requires`).
- Dependency behavior is user-overridable:
  - `--dependencies=auto|off` controls dependency auto-install.
  - `--dependency-versions=required|latest` controls whether dependency version requirements are pinned or allowed to float.
- `atlas validate` now checks all configured compatibility edges:
  - `mod -> mod` (required dependencies exist)
  - `mod -> loader`
  - `mod -> loader version`
  - `mod -> mc version`
  - `loader version -> mc version`
- Validation overrides are available:
  - `--check-dependencies=on|off`
  - `--check-dependency-versions=strict|off`
- `atlas publish --oidc-token` and `ATLAS_CI_OIDC_TOKEN` now authenticate CI
  requests using the `x-atlas-oidc-token` header on `/api/v1/ci/*` endpoints.
  They do not use runner service-token exchange.
- `atlas publish --deploy-token` and `ATLAS_PACK_DEPLOY_TOKEN` use pack deploy
  tokens (`atlas_pack_*`) via `x-atlas-pack-deploy-token`.

## Pointer compatibility metadata

`atlas mod add` now stores compatibility metadata directly in pointer TOMLs so
validation can run deterministically from repository state.

```toml
[compat]
minecraft = ["1.20.1"]
loaders = ["fabric"]
loader_versions = ["0.16.14"]

[[compat.requires]]
source = "modrinth"
project_id = "P7dR8mSH"
version = "LwYwM2QK"
```
