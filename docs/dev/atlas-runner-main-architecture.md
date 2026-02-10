# Atlas Runner — Architecture Document
Daemon-based Minecraft server deployment and management system

---

## Purpose

Atlas Runner enables users to deploy and manage Minecraft servers from Atlas Hub packs using a **CLI + Daemon** architecture:

- **CLI** (`atlas-runner`): User interface and configuration management
- **Daemon** (`atlas-runnerd`): Background service handling server lifecycle

Atlas Hub handles:
- pack building and dependency resolution
- version management and channel promotion
- artifact hosting and metadata

The runner handles:
- downloading and caching pack artifacts
- server provisioning and lifecycle management
- log aggregation and RCON access
- safe updates with rollback capability

---

# Design Goals

## Must have
- Single command setup with guided prompts
- No Docker dependency
- One server per daemon instance
- Deterministic installs with hash verification
- Safe auto-updates with caching
- RCON console support
- systemd integration
- Works on any Linux VPS

## Explicitly NOT in scope
- Multi-tenant hosting
- Web panel
- Fleet orchestration
- Pack building/mod resolution

---

# System Model

## Component Separation

### Atlas Hub (server side)
Provides:
- Pack metadata and build information
- Protocol Buffer blobs (.bin files)
- Dependency manifests with hashes
- Channel management (dev/beta/production)
- Player whitelist synchronization

### CLI (atlas-runner)
User-facing component responsible for:
- Interactive setup and configuration
- Daemon IPC communication
- Command parsing and validation
- User prompts (EULA, RAM, channel selection)

### Daemon (atlas-runnerd)
Background service responsible for:
- Server process supervision
- Pack blob application and updates
- Log streaming and aggregation
- RCON command execution
- Caching and performance optimization

---

# Distribution Model

Atlas publishes **Zstd-compressed Protocol Buffer blobs** (.bin files) containing:

## Blob Contents
- **Metadata**: Pack ID, version, Minecraft version, loader info
- **Manifest**: Dependency list with URLs, hashes, platform filters
- **File Map**: Virtual filesystem of configs and templates

## Dependency Resolution
Runner fetches artifacts directly from:
- Modrinth API
- CurseForge API
- Custom hosted assets
- Mojang/PaperMC servers

All downloads verified with SHA-256 hashes.

---

# High-Level Architecture

```
atlas-runner (CLI)
    ↓ IPC (Unix socket + Protobuf)
atlas-runnerd (Daemon)
├─ Supervisor (server lifecycle)
├─ Provisioner (pack application)
├─ Updater (polling + caching)
├─ IPC Handler (CLI communication)
├─ RCON Client (server console)
└─ Log Aggregator (stdout/stderr)
```

Pattern: **IPC-driven state machine**

CLI commands → IPC messages → Daemon actions → State updates

---

# IPC Protocol

## Transport
- **Socket**: Unix domain socket (`~/.atlas/runnerd/sockets/daemon.sock`)
- **Framing**: Length-prefixed Protocol Buffer messages
- **Encoding**: Protocol Buffers v3

## Message Types
```protobuf
message Request {
  oneof payload {
    Ping ping = 1;
    Start start = 2;
    Stop stop = 3;
    Status status = 4;
    LogsTail logs_tail = 5;
    RconExec rcon_exec = 6;
    SaveDeployKey save_deploy_key = 7;
  }
}

message Response {
  oneof payload {
    Pong pong = 1;
    Started started = 2;
    Stopped stopped = 3;
    Status status = 4;
    LogsTail logs_tail = 5;
    RconResult rcon_result = 6;
    DeployKeySaved deploy_key_saved = 7;
    Error error = 8;
  }
}
```

---

# Filesystem Layout

## Configuration Directory
```
~/.atlas/runnerd/
├── deploy.json          # Hub auth, pack config, user preferences
├── sockets/             # Unix domain sockets
├── runtime/             # Temporary files and staging
└── cache/               # Pack blobs and dependencies
    ├── blobs/           # .bin files by build ID
    └── artifacts/       # Downloaded JARs by hash
```

## Server Directory Structure
```
~/.atlas/runnerd/servers/default/
├── current/             # Active server files (replaceable)
├── staging/             # Update staging area
├── backups/             # World backups
├── logs/                # Server logs
└── data/                # Persistent data (world, configs)
    ├── world/           # Minecraft world files
    ├── config/          # Server configs (user-modified preserved)
    └── mods/            # Downloaded mod JARs
```

**Safety Rules:**
- `current/` and `staging/` are fully replaceable
- `data/world/`, `data/logs/` never deleted automatically
- `data/config/` preserved with `.atlas-new` for conflicts

---

# Commands

## Primary User Experience

### First Run Setup
```
atlas-runner up
```
Performs:
1. OAuth device code authentication
2. Channel selection (dev/beta/production)
3. RAM limit configuration (system-detected defaults)
4. EULA acceptance
5. Daemon startup and pack deployment

### Server Management
```
atlas-runner up          # Start server
atlas-runner down        # Stop server
atlas-runner restart     # Restart server
atlas-runner status      # Show server status
atlas-runner logs -f     # Stream server logs
```

### RCON Interface
```
atlas-runner exec "say hello world"    # Execute command
atlas-runner exec -it                  # Interactive console
```

### Daemon Management
```
atlas-runner daemon-status    # Check daemon health
atlas-runner daemon-logs     # View daemon internal logs
```

---

# Configuration Management

## Deploy Configuration (`deploy.json`)
```json
{
  "hubUrl": "https://hub.atlas.gg",
  "packId": "my-pack",
  "channel": "production",
  "deployToken": "secret-token",
  "maxRamMb": 4096,
  "shouldAutostart": true,
  "eulaAccepted": true
}
```

## Runtime Behavior
- **First run**: Interactive prompts for all settings
- **Subsequent runs**: Use cached configuration
- **Updates**: Configuration persists across pack updates

---

# Update Model

## Polling-Based Updates
- Daemon periodically polls Hub for channel changes
- Compares current build ID with latest
- Downloads and caches new blobs only when changed

## Safe Update Process
1. Download latest blob to cache
2. Stage new server files in `staging/`
3. Stop current server gracefully
4. Atomic swap `current/` ↔ `staging/`
5. Start new server
6. Health check with rollback on failure

## Caching Strategy
- **Pack blobs**: Cached by build ID, never re-downloaded
- **Dependencies**: Cached by SHA-256, verified on use
- **Build metadata**: Cached to avoid unnecessary API calls

---

# Provisioning System

## Pack Application
1. **Decompress**: Zstd decompression of .bin blob
2. **Deserialize**: Parse Protocol Buffer structure
3. **File Extraction**: Write embedded files to server directory
4. **Dependency Download**: Fetch and verify external JARs
5. **Platform Filtering**: Apply OS/architecture rules
6. **JVM Launch**: Generate classpath and arguments

## JVM Configuration
```bash
java -Xmx${max_ram_mb}m -Xms${max_ram_mb}m \
     -jar server.jar \
     --nogui \
     --port 25565
```

---

# Process Supervision

## Server Lifecycle
- **Start**: Spawn JVM process with configured arguments
- **Monitor**: Track process health and resource usage
- **Restart**: Auto-restart on crashes (configurable)
- **Stop**: Graceful shutdown with SIGTERM, force kill if needed

## Signal Handling
- **SIGTERM**: Graceful daemon shutdown, stop server
- **SIGINT**: Force daemon exit
- **SIGHUP**: Reload configuration

---

# systemd Integration

## Service Installation
```
atlas-runner systemd install
```

Creates service file with:
- User account execution (not root)
- Auto-restart on crashes
- Proper working directory
- Environment setup

## Service Management
```
systemctl --user start atlas-runnerd
systemctl --user status atlas-runnerd
journalctl --user -u atlas-runnerd -f
```

---

# Internal Modules (Rust Crates)

## Core Crates
- **`atlas-client`**: Hub API client with OAuth
- **`protocol`**: Protocol Buffer definitions and serialization
- **`mod-resolver`**: Dependency downloading and verification
- **`runner-core-v2`**: Shared types and utilities
- **`runner-ipc-v2`**: IPC protocol implementation
- **`runner-provision-v2`**: Pack application logic
- **`runner-v2-rcon`**: RCON client implementation
- **`runner-v2-utils`**: Common utilities

## Application Crates
- **`runner-v2`**: CLI application
- **`runnerd-v2`**: Daemon service

---

# Mental Model

Atlas Runner is:

**Docker Compose for Minecraft servers**
+ GitOps-style pack management
+ Daemon-based reliability
+ CLI-driven user experience
+ Safe update semantics

Not Pterodactyl (multi-tenant panel).  
Not Kubernetes (orchestration).  
Just a smart, reliable server launcher.