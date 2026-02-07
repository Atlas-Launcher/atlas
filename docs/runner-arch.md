# Atlas Runner — Architecture Document
Single‑server deployment agent for Atlas packs

---

## Purpose

`atlas-runner` is a lightweight system binary that allows a user to:

    atlas-runner launch <pack-id>

…and immediately run a Minecraft server on their own VPS.

Atlas Hub handles:
- pack building
- dependency resolution
- version pinning
- artifact metadata
- channel promotion

The runner handles:
- downloading artifacts
- assembling runtime
- running the server
- keeping it updated
- exposing lifecycle + RCON commands

The runner is not a platform, panel, or resolver.  
It is a small “rehydrator + supervisor”.

---

# Design Goals

## Must have
- Single command setup
- No Docker dependency
- One server only
- Deterministic installs (hash verified)
- Safe auto-updates
- RCON console support
- Works on any Linux VPS

## Explicitly NOT in scope
- Multi-tenant hosting
- Web panel
- Fleet orchestration
- Pack building

---

# System Model

## Separation of responsibilities

### Atlas Hub (server side)
Provides:
- Pack metadata
- Lockfile/blob
- Exact artifact list
- Channels (prod/beta/nightly)
- Hashes for every file

Already solved:
- dependency resolution
- mod compatibility
- version selection

### Runner (local machine)
Responsible only for:
- fetching exact files
- verifying hashes
- assembling disk layout
- launching Java
- supervising process
- redeploying on updates

Runner never decides versions.  
It only downloads what Atlas tells it to.

---

# Distribution Model (Blob + Lock Hybrid)

Atlas publishes:

## Blob
Contains:
- configs
- templates
- metadata
- runtime layout
- lockfile reference

Does NOT contain:
- mod jars (licensing)
- loader binaries
- server jars

## Lockfile
Specifies:
- exact mod files (provider IDs)
- loader + version
- server jar + version
- sha256 for everything

Runner fetches artifacts directly from:
- CurseForge
- Modrinth
- Mojang/Paper/etc

---

# High-Level Architecture

atlas-runner (single binary)
├─ auth
├─ hub client
├─ fetch/cache
├─ assemble runtime
├─ reconcile engine
├─ java supervisor
├─ rcon client
└─ CLI

Pattern:

Reconciler + Supervisor

desired state (lockfile)
→ plan
→ apply
→ run
→ repeat

---

# Filesystem Layout

## Root install (root)
```
/srv/atlas/server/
```

## User install
```
~/.local/share/atlas-runner/server/
```

## Structure
```
server/
  instance.toml
  state/state.json

  desired/
    lock.json

  cache/
    blobs/<hash>.tar.zst
    artifacts/<sha256>

  runtime/
    current/
    staging/

  data/
    world/
    mods/
    config/
    logs/
```

Rules:
- runtime/ = replaceable
- data/ = persistent
- never delete world/logs/config automatically

---

# Commands

## Primary UX

### Launch
```
atlas-runner launch <pack-id>
```

Performs:
1. auth
2. create instance dir
3. pull lockfile
4. download artifacts
5. assemble runtime
6. accept EULA
7. start server

Flags:
```
--accept-eula
--channel prod|beta|nightly
--memory 6G
--port 25565
--update auto|manual
--detach
```

---

## Lifecycle

### Start
```
atlas-runner up
```

### Stop
```
atlas-runner down
```

### Restart
```
atlas-runner restart
```

### Status
```
atlas-runner status
```

### Logs
```
atlas-runner logs -f
```

---

# RCON Interface

## Exec
```
atlas-runner exec "say hello"
```

## Interactive
```
atlas-runner exec -it
```

Interactive shell:
```
atlas>
```

---

# Update Model

## Channels
Each instance tracks one:
```
channel = "prod"
```

Runner periodically checks Atlas for latest blob/lock.

If changed → redeploy.

---

# Reconcile Algorithm

On update or start:

1. Fetch latest lockfile
2. Download + verify artifacts
3. Stage runtime
4. Stop server
5. Atomic swap
6. Start server
7. Health check
8. Rollback on failure

---

# Config Safety Policy

Local config precedence:

If file exists in `data/config` → leave untouched  
If missing → copy default  
If updated upstream → write `*.atlas-new`  
Force override:
```
atlas-runner up --force-config
```

---

# Process Supervision

Runner spawns:
```
java @args.txt
```

Responsibilities:
- log capture
- signal handling
- graceful shutdown

---

# systemd Integration

Optional service:

```
atlas-runner.service
```

Usage:
```
systemctl start atlas-runner
systemctl stop atlas-runner
journalctl -u atlas-runner -f
```

---

# Internal Modules

```
cli/
auth/
hub/
fetch/
cache/
assemble/
reconcile/
supervisor/
rcon/
```

Not present:
- dependency solver
- pack builder
- hosting panel
- multi-instance logic

---

# Mental Model

The runner is:

itzg-style startup script
+ Atlas sync
+ artifact fetch
+ RCON
+ safe updates

Not Pterodactyl.  
Not Kubernetes.  
Just a smart launcher.
