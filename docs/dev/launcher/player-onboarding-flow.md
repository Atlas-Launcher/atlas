# Player Onboarding Flow (Web -> Launcher)

This document describes the direct-replacement onboarding flow for players joining via invite.

## Goals

- Maximize invite funnel completion.
- Make launcher handoff deterministic.
- Reduce first-play failures by centralizing readiness + recovery.

## Web Invite Flow

1. Player opens `/invite?code=<inviteCode>`.
2. Player signs up/signs in.
3. Web accepts invite with `POST /api/v1/invites/accept`.
4. Response returns:
   - `packId` (legacy)
   - `pack` metadata
   - `onboarding.deepLink`
   - `onboarding.recommendedChannel`
5. Primary CTA opens `atlas://onboarding?...`.
6. If protocol handoff does not open launcher, user is shown installer fallback (`/download/app/installer/latest`).

## Launcher Intake and Intent Persistence

1. Launcher keeps existing `atlas://auth` / `atlas://signin` handling.
2. Non-auth deep links are parsed by onboarding deep-link parser.
3. Onboarding intent is persisted in settings as `pendingIntent`.
4. After Atlas pack sync, launcher matches remote instance by `packId`.
5. Launcher selects that instance, switches to library detail, and applies requested channel.
6. Launcher does not auto-install.
7. If readiness blockers exist, Launch Assist opens in readiness mode.
8. `pendingIntent` is cleared when handoff is fully applied.

## Unified Launch Assist

Launch Assist replaces split readiness + troubleshooter navigation.

- Readiness mode:
  - Shows blockers and one primary next action.
  - When sign-in/link blockers exist, Launch Assist switches to a sign-in-first state:
    - Auth-only checklist is shown.
    - Recovery tab is hidden until auth blockers are cleared.
- Recovery mode:
  - Runs diagnostics, surfaces top finding, supports fix actions.
  - Supports "Fix & Retry".
  - Re-runs diagnostics after each fix.
  - Allows support bundle generation and log access.

Modal layering behavior:
- Launch Assist and updater dialogs use full-window blur backdrops.
- macOS window controls and their control background remain visible above overlays.

Entry points:
- Title bar readiness button.
- Sidebar help action.
- Failure prompt.
- Settings activity action.

## Progress + First Success UX

- Task Center maps internal launch phases to:
  - Syncing pack
  - Preparing files
  - Starting Minecraft
- ETA appears when progress slope is stable.
- Indeterminate states show explicit estimation text.
- First confirmed launch success triggers compact success panel.
- Completion/dismissal is persisted in settings:
  - `firstLaunchCompletedAt`
  - `firstLaunchNoticeDismissedAt`
