# Launcher Developer Guide

Project: `apps/launcher` (+ `apps/launcher/src-tauri`)

## Responsibilities

- User auth UX and account-linking orchestration.
- Launch readiness evaluation.
- Troubleshooter UI and fix command integration.
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

- Readiness and launch Java checks use shared logic to reduce drift.
- Java override validation rejects invalid paths early and enforces compatibility checks.
- Download paths use retry/backoff with retry signal plumbing for user-visible progress.
- Troubleshooter focuses on post-install/launch issues; readiness gates launch blockers.
- Updater install clicks always open the updater dialog and surface actionable errors instead of silently no-oping.
- Signature verification failures are surfaced with a targeted message to check release signing key and launcher updater pubkey alignment.
- Launcher performs an automatic updater check on boot and then every hour while the app remains open.
- Updater `Update` handles from `@tauri-apps/plugin-updater` are stored as raw/shallow refs to avoid Vue proxying class instances with private fields.
- Tauri updater endpoints should target Distribution API routes:
- `/api/v1/launcher/updates/{os}/{arch}` (or channelized variant).
- Release workflow stamps `apps/launcher/src-tauri/tauri.conf.json` version from `launcher-vx.x.x` tags before `tauri build`.
- Because `apps/launcher/src-tauri` is a Cargo workspace member, CI release bundle output may be written to repo-root `target/release/bundle` (not only `apps/launcher/**/target/**/release/bundle`).

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
