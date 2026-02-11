# Atlas Executive Summary (Developer)

This summary is for platform engineers. It explains how Atlas operates today, what each project does, and how users are expected to use the system.

Estimated read time: about 15 minutes.

## 1. What Atlas Is

Atlas is a modpack platform built around one core model:

- Source in Git.
- Distribution in a compiled binary artifact.

Creators maintain readable config in Git. Runtime clients (launcher for players, runner for server hosts) consume built artifacts that are immutable and channel-addressable.

## 2. Core Product Surfaces

### Web (Hub)

`apps/web` is the control plane and API boundary.

It provides:
- authentication and account linking,
- pack ownership and membership controls,
- build/channel metadata,
- storage presign/download indirection,
- launcher and runner API endpoints,
- CI upload completion and release flow.

### Launcher

`apps/launcher` is a player client (Vue + Tauri).

It provides:
- sign-in and account linking flows,
- launch readiness checks,
- troubleshooter and fix actions,
- Minecraft runtime hydration (files, assets, loader/runtime),
- launcher self-updater UX,
- launch orchestration and log capture.

### Runner-v2 + Runnerd-v2

`apps/runner-v2` and `apps/runnerd-v2` are server-host tooling.

- `runner-v2`: user-facing CLI (`atlas-runner`) for auth/control operations.
- `runnerd-v2`: long-running daemon (`atlas-runnerd`) that applies artifacts and supervises process lifecycle.

The CLI talks to the daemon over local IPC (`runner-ipc-v2`), while the daemon talks to Hub APIs through `atlas-client`.

### CLI (Pack Authoring/Deploy)

`apps/cli` (`atlas`) is creator/operator tooling for pack lifecycle work:
- init/reinit pack config,
- add/remove/list assets,
- build artifacts,
- push/pull,
- CI workflow sync,
- deploy/complete build flow.

## 3. Distribution Model and Immutability

Atlas compiles pack source into a binary artifact (current extension in codepaths is `.atlas`).

Important invariants:
- Builds are immutable once published.
- Channels (`dev`, `beta`, `production`) are mutable pointers.
- Clients should rely on channel pointer movement for promotion, not artifact mutation.

This model allows:
- deterministic installs,
- reproducible rollback/promotion,
- clear separation between authoring source and runtime payload.

## 4. How Users Actually Use Atlas

### Creator/Server Host Flow

1. Create/update pack source in Git.
2. Build and upload artifact via CLI/CI.
3. Move channel pointers via Hub/API workflows.
4. For servers, runner-v2/runnerd-v2 pull and apply builds.

### Player Flow

1. Sign in to Atlas + Microsoft.
2. Link accounts.
3. Launcher checks readiness and blocks avoidable launch failures.
4. Launcher fetches channel build metadata and hydrates runtime.
5. Player launches Minecraft from hydrated instance.

## 5. Data Contracts and Trust Boundaries

### Web API is the system boundary

External tools (launcher, runner, CLI, CI) should treat web endpoints as the contract layer.

### Local state

- Launcher stores local settings/session state and instance runtime data.
- Runnerd stores deploy/runtime state in its runtime path.

### Security model in practice

- User bearer/session for user-scoped API access.
- Runner access tokens for runner-scoped endpoints.
- CI OIDC or authorized user token for CI build publish paths.
- Storage access mostly via presigned URLs or signed token indirection.

## 6. Launch and Provisioning Strategy

### Launcher

Provisioning includes:
- version manifest resolution,
- client jar/assets/libraries hydration,
- loader install/profile handling,
- Java runtime resolution and validation,
- runtime integrity checks and retries.

Diagnostics and launch readiness are now tied to launch source-of-truth Java checks to avoid “ready but fails on launch” drift.

### Runner provisioning

`runner-provision-v2` is the active Java/provisioning implementation.

Current state:
- Java runtime install with checksum verification,
- extracted runtime hash verification (`java.hash`),
- corruption detection and reinstall behavior,
- shared Java installer API now also used by legacy runner wrapper.

## 7. Platform Targets

Official support is intentionally narrow and explicit:
- Launcher: Windows + macOS + Linux (Linux best effort).
- Runner CLI + Runnerd: macOS + Linux (including WSL).
- Web: Vercel deployment target.
- Runner v1 (`apps/runner`) is legacy and unsupported.

If it is not listed, it is unsupported.

## 8. Current Engineering Direction

The current code direction emphasizes:
- reducing divergence between diagnostics and launch/provisioning logic,
- tightening Java/runtime validation,
- improving self-serve recovery (readiness + troubleshooter),
- enforcing explicit support boundaries,
- keeping contracts stable as UI/UX evolves.

## 9. How to Work Safely in This Repo

1. Start from executable truth (`apps/*`, `crates/*`) before docs.
2. Keep channel/build immutability guarantees intact.
3. Prefer shared runtime/provisioning logic over duplicate implementations.
4. Treat web route contracts as public interfaces.
5. Validate with targeted checks/tests in touched projects before shipping.

## 10. What to Read Next

- `../launcher/README.md`
- `../web/README.md`
- `../web/api-spec.md`
- `../runner-v2/README.md`
- `../runnerd-v2/README.md`
- `../cli/README.md`
