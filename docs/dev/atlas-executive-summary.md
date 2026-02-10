# Atlas Executive Summary

## What is Atlas?

Atlas is a **Source-in-Git, Distribution-in-Binary** platform for Minecraft modpack management. It enables creators to manage modpacks in Git repositories while distributing optimized binary blobs to players and servers.

## Core Architecture (5-Minute Overview)

### The Source-in-Git Model
- **Pack Definition**: Modpacks defined in `.toml` files
- **Version Control**: Standard Git workflows for collaborative development
- **CI/CD Integration**: Automated building via GitHub Actions
- **Immutable Builds**: Each Git commit produces a versioned, immutable build artifact

### The Distribution-in-Binary Model
- **Protocol Buffers**: Efficient serialization of pack metadata and manifests
- **Zstd Compression**: High-compression binary blobs for fast distribution
- **Content Addressing**: SHA-256 based verification and caching
- **Platform Filtering**: Cross-platform compatibility with native optimizations

### System Components
```
Creator Git Repo → GitHub Actions → Binary Blob → Cloudflare R2
                                      ↓
Player/Server ← Downloads ← Atlas Hub API ← Authentication
```

## End-User Experience (5-Minute Overview)

### For Players (Minecraft Gamers)
1. **Download Atlas Launcher** → Install like any other game launcher
2. **Get Pack Invitation** → Creator sends you a link via Discord/email
3. **Click Play** → Launcher handles everything: mods, Java, Minecraft installation
4. **Play Modded Minecraft** → Seamless experience, automatic updates

**Key Value**: No technical knowledge required. Just download, get invited, play.

### For Creators (Modpack Developers)
1. **Create GitHub Repo** → Standard Git repository, very similar to packwiz
2. **Add pack.toml** → Declare mods, configs, server settings
3. **Test Locally** → Use `atlas` CLI to build and test packs locally
4. **Push to Git** → GitHub Actions automatically builds and publishes
5. **Invite Players** → Send invitation links, manage access

**Key Value**: Git-native workflow with local CLI testing. Focus on content, not infrastructure.

### For Server Hosts (Multiplayer Operators)
1. **Authenticate** → Link your Atlas creator account
2. **Run One Command** → `atlas-runner up` downloads and starts server
3. **Server Runs Automatically** → Auto-updates, backups, monitoring
4. **Players Connect** → Standard Minecraft multiplayer

**Key Value**: Zero server administration. Just deploy and manage community.

## Why Should You Care? (5-Minute Overview)

### Interesting Technical Challenges
- **Cross-Platform Distribution**: Windows, macOS, Linux with native performance
- **Dependency Resolution**: Complex mod compatibility and conflict resolution
- **Binary Optimization**: Zstd compression + delta updates for efficient distribution
- **Real-Time Systems**: Live server monitoring, auto-scaling, health checks
- **Security**: Cryptographic verification, sandboxed builds, secure distribution

### Modern Tech Stack
- **Rust**: High-performance systems programming for CLI tools and services
- **TypeScript/React**: Modern web dashboard with great developer experience
- **Protocol Buffers**: Efficient, typed, cross-language serialization
- **Distributed Systems**: CDN, caching, background processing, real-time updates

### Impact & Scale
- **Minecraft Ecosystem**: One of the largest gaming modding communities
- **Real User Problems**: Solving actual pain points in modpack distribution
- **Growing Platform**: Building infrastructure that scales with user adoption
- **Open Source**: Contributing to the broader Rust and web development communities

## Development Quick Start

### Key Technologies You'll Work With
- **Rust**: CLI tools, services, cross-platform code
- **TypeScript**: Web dashboard, API clients
- **Protocol Buffers**: Data serialization and API contracts
- **GitHub Actions**: CI/CD and automation
- **Cloudflare R2**: Object storage and CDN

### Getting Started
1. **Clone the monorepo**: `git clone https://github.com/atlas-launcher/atlas-launcher`
2. **Install tools**: Rust, Node.js, pnpm
3. **Run locally**: `pnpm install && cargo build`
4. **Read the code**: Start with `apps/runner-v2/src/main.rs` or `apps/web/`

### Current Architecture Focus
- **Source-in-Git Model**: Git-based pack management (mostly complete)
- **Binary Distribution**: Protocol Buffers + Zstd (core implemented)
- **Cross-Platform Runtime**: Rust-based CLI and services (mature)
- **Web Dashboard**: Creator tools and analytics (growing)
- **Real-Time Features**: Live updates, monitoring (emerging)

### Secondary Personas
- **Players**: End users wanting easy modded Minecraft access
- **Platform Integrators**: Developers building tools and services