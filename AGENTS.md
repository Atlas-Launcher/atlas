# AGENTS

## Purpose
This document summarizes the Atlas technical architecture so agents can align changes with the intended system design.

## System Overview
Atlas is a platform for managing and distributing Minecraft modpacks using a Source-in-Git, Distribution-in-Binary model.
- Creators manage packs in Git with TOML and text configs.
- GitHub Actions compiles repos into a single optimized binary blob.
- The backend manages access, versions, and hosting.
- The launcher consumes the blob to hydrate the game directory and download dependencies.

## Component Architecture
- Management Hub (Next.js on Vercel)
  - Web dashboard for Admins, Creators, Players.
  - REST API gateway for CLI, CI, and Launcher.
  - Identity provider for email/password with optional GitHub OAuth linking.
  - Release manager for channel pointers (Dev, Beta, Production).
- Builder CLI & CI (Rust + GitHub Actions)
  - CLI used locally and in CI to compile distribution blobs.
  - CI pushes blobs to the Hub using deploy tokens.
- Object Storage (Cloudflare R2 + CDN)
  - Immutable blob storage for .bin artifacts.
  - Asset hosting for custom JARs/assets.

## Distribution Format (.bin)
- Zstd-compressed Bincode blob.
- Metadata and manifest include pack ID, version, Minecraft version, loader, dependency URLs, hashes, and platform filters.
- Embedded payload is a virtual filesystem map of relative paths to bytes.

## Identity and Access Management
- Roles: System Admin, Pack Creator, Player.
- Users can link GitHub; tokens are stored encrypted.
- Deploy tokens authorize CI uploads.
- Channel permissions gate Dev/Beta access.

## Version and Release Management
- Builds are immutable artifacts produced per Git commit.
- Channels are mutable pointers to builds.
- Dev auto-updates on each push; promotion moves channel pointers.

## Core Workflows
- Pack creation and deployment
  1. Import GitHub repo and inject CI workflow.
  2. Commit pack changes to Git.
  3. CI builds the .bin blob.
  4. CLI posts blob to Hub; Hub updates Dev channel.
- Launcher hydration
  1. Fetch active build for a channel.
  2. Download the .bin blob.
  3. Decompress (Zstd) and deserialize (Bincode).
  4. Write text/config files; download filtered dependencies; verify hashes.
  5. Launch JVM with computed classpath.

## Infrastructure Notes
- Backend: Next.js edge-compatible.
- Database: Managed PostgreSQL.
- Storage: Cloudflare R2.
- Serialization: Bincode.
- Compression: Zstd (level 19+).
- CLI/Launcher: Rust for low memory usage and fast hashing.

## Agent Guidance
- Keep changes aligned with the Source-in-Git, Distribution-in-Binary model.
- Preserve immutability of builds and mutability of channels.
- Favor fast, deterministic compilation and hydration paths.
- Maintain platform filter logic and hash verification for dependencies.

## Code Quality and UX
- Keep code clean and readable; follow best practices in logic, UI, and UX.
- One purpose per file: each file should have a single, focused responsibility.
- Prefer simple, straightforward end-user flows without sacrificing usability.

## Shadcn/ui
- Use shadcn/ui components for consistent styling and behavior.
- You can install them with either `pnpm dlx shadcn-vue@latest add <component>` or `pnpm dlx shadcn@latest add <component>`, if working in react.