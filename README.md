# MC Launchpad (Barebones)

A tiny Tauri + Vue launcher that only does Microsoft device-code sign-in and launches vanilla Minecraft.

## What it does
- Microsoft device code login
- Downloads the selected Minecraft version + libraries + assets
- Launches Minecraft with a minimal JVM profile

## Setup
1. Install dependencies:
   - `pnpm install`
2. Run in dev:
   - `pnpm tauri:dev`

## Microsoft client ID
You must supply a Microsoft application (client) ID in the UI. Create an Azure app registration and enable device-code flow for public clients.

## Notes
- Default game directory is the platform `.minecraft` folder.
- This is intentionally minimal and does not manage mods, profiles, or multiple instances.
