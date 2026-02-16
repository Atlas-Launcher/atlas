# Repository Map

This map is code-first and reflects current workspace members.

## Top-Level Projects

- `apps/cli`: Atlas pack authoring and deploy CLI (`atlas`).
- `apps/launcher`: Desktop launcher frontend (Vue) + backend (`src-tauri`).
- `apps/web`: Atlas Hub web app and API (Next.js).
- `apps/runner-v2`: Runner operator CLI (`atlas-runner`).
- `apps/runnerd-v2`: Runner daemon (`atlas-runnerd`).
- `apps/runner`: Legacy runner v1 (unsupported).

## Repository governance

- `LICENSE`: Atlas Proprietary License (non-commercial use only; no re-hosting).

## Shared Rust Crates

- `crates/protocol`: pack/protobuf model + config handling.
- `crates/atlas-client`: Hub client used by CLI/runner components.
- `crates/mod-resolver`: mod/provider resolution.
- `crates/runner-core-v2`: runner protocol payload types.
- `crates/runner-ipc-v2`: runner CLI <-> daemon IPC.
- `crates/runner-provision-v2`: apply/provision + Java install logic.
- `crates/runner-v2-utils`: paths/runtime helpers.
- `crates/runner-v2-rcon`: RCON helper logic.

## Key Runtime Data Flows

1. Creator edits pack source in Git.
2. CLI/CI builds immutable binary artifact (`.atlas`).
3. Web API stores artifact and moves channel pointers.
4. Launcher resolves channel -> build -> download URL and hydrates instance.
5. Runner-v2/runnerd-v2 resolve deploy context and apply server runtime.
