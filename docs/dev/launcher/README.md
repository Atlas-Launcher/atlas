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
  - `atlas://onboarding?source=invite&packId=<id>&channel=<dev|beta|production>`
- Launcher auth uses provider-specific flows:
  - Microsoft sign-in uses Authorization Code + PKCE via system browser
    (`begin_deeplink_login` + `complete_loopback_login`) with a local
    loopback callback listener (`http://127.0.0.1:<port>/callback`) and falls back to
    device code (`start_device_code` + `complete_device_code`) only when browser
    open fails.
  - Atlas Hub sign-in uses device code (`start_atlas_device_code` +
    `complete_atlas_device_code`).
  - Atlas device token polling accepts both OAuth snake_case and camelCase
    token fields for compatibility with auth provider response variants.
  - If Atlas `/oauth2/userinfo` rejects a device token, launcher falls back to
    a local Atlas profile identity and continues using the valid bearer token,
    enriching Mojang fields via Hub API when available.
  - Device-code paths open `verification_uri_complete` when provided; otherwise
    they open `verification_uri` with `user_code` appended when supported by the
    provider.
  - For Microsoft device-code fallback, when `verification_uri_complete` is not
    provided, launcher explicitly surfaces the device code in status and
    attempts to copy it to the clipboard.
  - On successful Microsoft sign-in completion, launcher explicitly focuses the
    main window via backend command (`focus_main_window`) with frontend fallback.
  - Auth diagnostics are written to launcher telemetry log at
    `<data_dir>/atlas/launcher.log` (for example:
    `~/Library/Application Support/atlas/launcher.log` on macOS) with Atlas
    device-code start/poll/complete events.
- Onboarding handoff persists pending intent and applies it after pack sync by selecting matching Atlas instance.
- Readiness and launch Java checks use shared logic to reduce drift.
- Java override validation rejects invalid paths early and enforces compatibility checks.
- Download paths use retry/backoff with retry signal plumbing for user-visible progress.
- Launch Assist surfaces separate modals:
  - `Account status` modal for sign-in/account-link blockers (opened from the sign-in status button).
  - `Recovery` modal for troubleshooting findings/fixes/support bundle (opened from `?` help and failure prompts).
  - These are independent modal flows instead of tab-switching within one shared modal.
  - Account status checklist uses icon/color state for blockers without a separate
    "blocked" badge label.
  - Account status modal hides the `filesInstalled` and `javaReady` checklist rows
    from display while preserving underlying readiness/blocker logic.
  - Account status primary action CTA is aligned to the right action bar for clearer
    progression emphasis.
  - Wizard close actions are locked while Atlas sign-in, Microsoft sign-in, or
    account-link readiness is still unresolved (X is hidden and Escape close
    path is blocked).
  - In close-locked state, the footer divider/chin is removed when no footer
    controls are available.
  - Footer `Close` button is removed; top-right `X` is the only close control.
    In account status mode, sign-out is inline on the same action row as the primary
    next-step CTA.
  - While sign-in is in progress, readiness actions prevent starting additional
    sign-in attempts, and Atlas/Microsoft blockers expose a "Show code" path
    (copy + open verification page) similar to account-link code reveal.
- Instance detail view keeps setup focused:
  - `Setup` tab is game setup only.
  - `Profile` tab contains profile/runtime override settings.
- Updater uses an in-banner flow (no separate overlay dialog) and surfaces actionable errors inline.
- Signature verification failures are surfaced with a targeted message to check release signing key and launcher updater pubkey alignment.
- Launcher performs an automatic updater check on boot and then every hour while the app remains open.
- On boot, while the prelaunch loading window is open, launcher checks for
  updates and runs download/install automatically before main UI bootstrap. When
  install succeeds, launcher restarts immediately into the updated build.
- Restart uses an explicit cross-platform relaunch path (spawn current
  executable, then exit) so update apply reliably reopens on Windows, macOS,
  and Linux.
- Updater is disabled in development builds (`import.meta.env.DEV`): no auto
  check/install runs, and manual updater actions are no-ops with status
  messaging.
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
  - Troubleshooter no longer emits a "Game files need to be installed" finding
    for fresh/uninstalled profiles; repair/corruption findings are reserved for
    installed profiles with failure/corruption signals.
  - Java version/runtime compatibility is validated during launch, not treated as
    a pre-launch readiness blocker.
- First confirmed launch success is persisted and shown via a compact first-run success panel.
- Tauri updater endpoints should target Distribution API routes:
- `/api/v1/launcher/updates/{os}/{arch}` (or channelized variant).
- Release workflow stamps `apps/launcher/src-tauri/tauri.conf.json` version from `launcher-vx.x.x` tags before `tauri build`.
- Because `apps/launcher/src-tauri` is a Cargo workspace member, CI release bundle output may be written to repo-root `target/release/bundle` (not only `apps/launcher/**/target/**/release/bundle`).
- Microsoft OAuth client ID is injected at compile time from `ATLAS_MS_CLIENT_ID`
  (GitHub Actions secret: `ATLAS_MS_CLIENT_ID`) rather than hardcoded in source.
- Launcher Advanced settings no longer exposes Microsoft client ID or Atlas Hub
  URL inputs, and Settings no longer uses a manual Save button. Runtime/theme
  edits persist automatically.
- Default JVM memory now uses a one-time RAM profile migration:
  - 6 GB on 8 GB systems.
  - 8 GB on 16 GB systems.
  - 12 GB on 24 GB+ systems.
  - Migration completion is tracked in settings (`defaultMemoryProfileV1Applied`)
    so existing installs apply the new baseline once after upgrade.
- Runtime settings now expose a memory slider + numeric input with a
  **Use recommended** action:
  - Slider and numeric input stay synchronized bidirectionally.
  - Slider shows notable memory markers at 2, 4, 6, 8, 12, 16, 24, and 32 GB.
  - Slider endpoint labels are shown in GB using floor rounding
    (for example, 14.5 GB displays as 14 GB) and use the same visual style as notable ticks.
  - Slider min/max endpoint marks are rendered inline with the same labeled tick row.
  - Recommended memory in UI is shown in floored GB.
  - Numeric memory entry keeps raw numeric input, with `MB` rendered as a suffix outside the field.
  - Slider movement snaps to notable marker values only within a tight proximity window.
  - Slider also renders subtle unlabeled ticks at every allowed 512 MB step.
  - Memory entries are validated and snapped to 512 MB steps.
  - Values round up to the nearest valid step unless that would exceed
    `system RAM - 2 GB`, in which case they round down to the cap.
  - On systems with 34 GB+ RAM, slider/input max is capped at 32 GB.
  - Helper copy under the slider is system-aware and surfaces active limits
    (for example, OS reserve and high-memory max cap), with system RAM shown in GB (floored).
- Profile runtime overrides now use the same shared RAM selector UI as launcher
  defaults (same marks, snapping, limits, and validation behavior).

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
