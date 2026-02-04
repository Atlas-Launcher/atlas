# Atlas Launcher

A tiny Tauri + Vue Minecraft launcher.

## What it does (currently)
- Login with Microsoft account
- Download the selected Minecraft version + libraries + assets
- Launch Minecraft with a minimal JVM profile

## Setup
1. Install dependencies:
   - `pnpm install`
2. Run in dev:
   - `pnpm tauri:dev`
   - or `pnpm tauri:dev:device-code` if you don't want to use the deeplink flow (good for macOS).
3. Build:
   - `pnpm tauri:build`

## Future Plans
Not in any particular order:
- Loader support (Fabric, Quilt, Forge, NeoForge)
- Mod management (CurseForge, Modrinth, manual)
- Modpack management and distribution
- Multi-instance support
- Multi-account support


## Microsoft client ID
You must supply a Microsoft application (client) ID in the UI. Create an Azure app registration and enable device-code flow for public clients.
