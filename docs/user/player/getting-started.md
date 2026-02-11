# Player Guide

This guide is for players using Atlas Launcher.

## 1. Platform Support

Launcher supports:
- Windows
- macOS
- Linux (best effort)

If your platform is not listed, it is unsupported.

## 2. Sign-in and Link Flow

To become launch-ready:
1. Sign in to Atlas account.
2. Sign in with Microsoft account.
3. Complete account link so launcher and Atlas identity match.

The launcher readiness flow is designed to block launch until blockers are resolved.

## 3. Install and Launch

Launcher will:
- resolve pack/version data,
- hydrate required files/assets/libraries,
- verify and resolve Java runtime requirements,
- install required loader/runtime metadata,
- launch Minecraft with computed arguments.

## 4. Updates

Launcher can check for launcher app updates and install them through in-app updater flow.

## 5. If Launch Fails

Use launcher troubleshooter for post-install/runtime issues.
Readiness handles pre-launch blockers (auth/link/readiness).
