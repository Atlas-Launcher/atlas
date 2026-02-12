# Player Guide

This guide is for players using Atlas Launcher.

## 1. Platform Support

Launcher supports:
- Windows
- macOS
- Linux (best effort)

If your platform is not listed, it is unsupported.

## 2. Invite to Launcher Flow

When you open an invite link in the web app:
1. Create or sign in to your Atlas account.
2. Invite acceptance happens automatically.
3. Select **Open Atlas Launcher** on the invite page.
4. If launcher protocol handoff fails, use **Download Atlas Launcher** and retry.

Launcher opens with your invited pack preselected. It does not auto-install.

## 3. Sign-in and Link Readiness

Atlas uses a single Launch Assist flow to get you launch-ready:
1. Sign in to Atlas.
2. Sign in with Microsoft.
3. Complete account linking so IDs match.

Launch Assist shows one next action at a time for blockers.

## 4. Install and Launch

Launcher will:
- resolve pack/version data,
- hydrate required files/assets/libraries,
- verify and resolve Java runtime requirements,
- install required loader/runtime metadata,
- launch Minecraft with computed arguments.

Progress is shown as:
- Syncing pack
- Preparing files
- Starting Minecraft

## 5. Updates

Launcher can check for launcher app updates and install them through in-app updater flow.

## 6. First Launch Completion

After the first confirmed successful launch, Atlas shows a compact success panel with quick actions.

## 7. If Launch Fails

Open **Launch Assist** from the sidebar, settings activity card, or failure prompt.
It includes readiness guidance, recovery actions, diagnostics reruns, logs, and support bundle generation.
