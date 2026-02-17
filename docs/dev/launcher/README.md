# Launcher Developer Guide

Project: `apps/launcher` (+ `apps/launcher/src-tauri`)

## Responsibilities

- User auth UX and account-linking orchestration.
- Invite onboarding deep-link intake and pack preselection.
- Launch readiness evaluation and recovery orchestration.
- Launch Assist UI and fix command integration.
- Minecraft runtime hydration and launch process management.
- Launcher self-update UX (Tauri updater plugin).

## Architecture (Current)

- Frontend: Vue app (`src/`) with component-driven flows.
- Backend: Tauri commands (`src-tauri/src/commands/*`) invoking launcher/diagnostics/library modules.

Key backend modules:
- `launcher/*`: version resolution, downloads, Java runtime handling, loader install paths.
- `diagnostics/*`: readiness/troubleshooter/fix/reporting logic.
- `library/*`: local instance metadata and sync operations.

## Notable Current Behaviors

- Launcher supports onboarding protocol URLs:
  - `atlas://auth`
  - `atlas://signin`
  - `atlas://onboarding?source=invite&packId=<id>&channel=<dev|beta|production>`
- Onboarding handoff persists pending intent and applies it after pack sync by selecting matching Atlas instance.
- Readiness and launch Java checks use shared logic to reduce drift.
- Java override validation rejects invalid paths early and enforces compatibility checks.
- Download paths use retry/backoff with retry signal plumbing for user-visible progress.
- Launch Assist is unified:
  - Readiness phase for pre-launch blockers.
  - Recovery phase for post-readiness findings/fixes/support bundle.
- Instance detail view keeps setup focused:
  - `Setup` tab is game setup only.
  - `Profile` tab contains profile/runtime override settings.
- Updater uses an in-banner flow (no separate overlay dialog) and surfaces actionable errors inline.
- Signature verification failures are surfaced with a targeted message to check release signing key and launcher updater pubkey alignment.
- Launcher performs an automatic updater check on boot and then every hour while the app remains open.
- Updater `Update` handles from `@tauri-apps/plugin-updater` are stored as raw/shallow refs to avoid Vue proxying class instances with private fields.
- Atlas remote profile sync now applies deduping in both Tauri and Vue settings hydration:
  - Tauri `fetch_atlas_remote_packs` suppresses duplicate `pack_id` rows.
  - Vue settings normalization infers `source: "atlas"` when `atlasPack` metadata exists, normalizes remote pack identifiers, and collapses duplicate atlas instances by `packId` and duplicate IDs during load and remote sync.
  - This prevents legacy settings payloads from showing duplicate pack cards in launcher.
- Updater banner is rendered in an isolated high-z layer with an opaque card surface so underlying form labels do not bleed through while scrolling.
- Task center progress maps launcher internals to player-facing stages:
  - Syncing pack
  - Preparing files
  - Starting Minecraft
- Install-to-play flow:
  - When users click **Install** in instance/library views, launcher now starts
    play automatically as soon as install/sync completes successfully.
  - If launch prerequisites are still unmet (for example account link), launcher
    keeps install success and shows a readiness status message instead of
    launching.
- Successful launch banner:
  - Launcher now shows the success banner after every successful launch event
    (not only first launch), with headline text `Launch complete`.
- Diagnostics guardrails:
  - Readiness/troubleshooter does not classify "install corruption" when profile
    files are not installed yet.
  - Java version/runtime compatibility is validated during launch, not treated as
    a pre-launch readiness blocker.
- First confirmed launch success is persisted and shown via a compact first-run success panel.
- Tauri updater endpoints should target Distribution API routes:
- `/api/v1/launcher/updates/{os}/{arch}` (or channelized variant).
- Release workflow stamps `apps/launcher/src-tauri/tauri.conf.json` version from `launcher-vx.x.x` tags before `tauri build`.
- Because `apps/launcher/src-tauri` is a Cargo workspace member, CI release bundle output may be written to repo-root `target/release/bundle` (not only `apps/launcher/**/target/**/release/bundle`).
- Microsoft OAuth client ID is injected at compile time from `ATLAS_MS_CLIENT_ID` (GitHub Actions secret: `ATLAS_MS_CLIENT_ID`) rather than hardcoded in source.

## Command Surface (Tauri)

Main invoke handlers are registered in:
- `apps/launcher/src-tauri/src/main.rs`

Representative command groups:
- Settings: get/update default dirs/settings.
- Auth: sign-in, restore session, sign-out, launcher link session.
- Launch: `launch_minecraft`, `download_minecraft_files`.
- Diagnostics: readiness, troubleshooter, apply fix, repair, support bundle.
- Library: versions/mods/pack sync and related actions.

## Platform Notes

Supported:
- Windows, macOS, Linux (Linux best effort)

If a platform is not explicitly listed, it is unsupported.
