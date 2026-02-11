# Atlas Domain Map

Use this map to pick high-value context files fast.

## Workspace Topology

- `Cargo.toml`: Rust workspace members and crate boundaries.
- `package.json`: JS workspace scripts and tooling.
- `pnpm-workspace.yaml`: pnpm workspace layout.
- `turbo.json`: task graph for build/test/lint pipelines.

## System and Architecture Context

- `AGENTS.md`
- `docs/dev/atlas-system-architecture.md`
- `docs/dev/atlas-executive-summary.md`

Use these for high-level intent before touching implementation files.

## Desktop Launcher (Vue + Tauri)

Executable sources:
- `apps/launcher/src/main.ts`
- `apps/launcher/src/App.vue`
- `apps/launcher/src-tauri/src`
- `apps/launcher/src-tauri/tauri.conf.json`
- `apps/launcher/package.json`

Supporting docs:
- `apps/launcher/README.md`
- `docs/dev/atlas-launcher-developer-guide.md`

## Web Hub and API (Next.js)

Executable sources:
- `apps/web/app/api/v1/**/route.ts`
- `apps/web/app/api/auth/[...all]/route.ts`
- `apps/web/lib/auth`
- `apps/web/lib/db/schema.ts`
- `apps/web/drizzle/*.sql`
- `apps/web/package.json`

Supporting docs:
- `docs/dev/atlas-web-api-reference.md`
- `docs/dev/atlas-system-architecture.md`

## Runner CLI + Daemon (Rust)

Executable sources:
- `apps/runner-v2/src`
- `apps/runnerd-v2/src`
- `crates/runner-core-v2/src`
- `crates/runner-ipc-v2/src`
- `crates/runner-provision-v2/src`
- `crates/runner-v2-utils/src`
- `crates/runner-v2-rcon/src`

Supporting docs:
- `docs/dev/atlas-runner-main-architecture.md`
- `docs/dev/atlas-runner-technical-documentation.md`
- `docs/dev/atlas-runner-cli-developer-guide.md`
- `docs/dev/atlas-runner-daemon-developer-guide.md`
- `docs/dev/runner-core-v2-developer-guide.md`
- `docs/dev/runner-ipc-v2-developer-guide.md`
- `docs/dev/runner-provision-v2-developer-guide.md`
- `docs/dev/atlas-runner-api-reference.md`

## Distribution Format and Protocol

Executable sources:
- `crates/protocol/proto/atlas.proto`
- `crates/protocol/build.rs`
- `crates/protocol/src`
- `docs/dev/atlas-runner-data-formats.md`

Supporting docs:
- `docs/dev/protocol-developer-guide.md`

## Hub Client and Mod Resolution

Executable sources:
- `crates/atlas-client/src`
- `crates/mod-resolver/src`

Supporting docs:
- `docs/dev/atlas-client-developer-guide.md`
- `docs/dev/mod-resolver-developer-guide.md`

## Distribution, Releases, and Downloads

Executable sources:
- `apps/web/app/download`
- `apps/web/lib/releases.ts`
- `apps/web/lib/installer-assets.ts`
- `apps/web/lib/storage`
- `apps/cli/src/commands/deploy.rs`
- `apps/cli/src/commands/ci.rs`

Supporting docs:
- `docs/user/testing-dev-builds-with-atlas-runner.md`
- `docs/user/atlas-launcher-user-guide.md`
- `docs/user/atlas-runner-user-guide.md`

## User-Facing Behavior

Use these when output must match product behavior seen by creators/players/operators:

- `docs/user/atlas-end-user-stories.md`
- `docs/user/atlas-launcher-user-guide.md`
- `docs/user/atlas-runner-user-guide.md`
