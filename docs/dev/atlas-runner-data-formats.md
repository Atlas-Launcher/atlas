# Atlas Runner Data Formats

This document details the data formats used in Atlas Runner, including the in-repository pack format and the binary distribution formats.

## In-Repository Pack Format

Creators manage Minecraft server packs in Git repositories using TOML configuration files and standard Minecraft server files.

### Repository Structure

```
my-pack/
├── pack.toml           # Pack metadata and dependencies
├── server.properties   # Minecraft server configuration
├── ops.json           # Operator list
├── whitelist.json     # Player whitelist
├── config/            # Mod configuration files
│   ├── mod1.toml
│   ├── mod2.json
│   └── ...
└── mods/              # Local mod files (optional)
    └── custom-mod.jar
```

### Pack Configuration (`pack.toml`)

The main configuration file defining pack metadata, dependencies, and settings.

#### Schema

```toml
[pack]
id = "my-server-pack"           # Unique pack identifier
name = "My Server Pack"         # Human-readable name
version = "1.0.0"              # Pack version (semantic versioning)
minecraft_version = "1.20.1"   # Target Minecraft version
loader = "fabric"              # Mod loader: "fabric", "forge", "neoforge", "vanilla"
loader_version = "0.15.7"      # Loader version (when applicable)
description = "A custom Minecraft server pack"

[dependencies]
# Mod dependencies from Modrinth
fabric_api = { modrinth = "P7dR8mSH", version = "0.92.0" }
sodium = { modrinth = "AANobbMI", version = "0.5.3" }

# CurseForge dependencies
worldedit = { curseforge = 225608, version = "7.2.15" }

# Direct URL dependencies
custom_mod = { url = "https://example.com/mod.jar", hash = "sha256:..." }

[[mods]]
# Alternative array syntax for mods
name = "LuckPerms"
source = { curseforge = 341284 }
version = "5.4.102"
side = "server"  # "both", "client", "server"

[overrides]
# Override default server properties
max_players = 50
difficulty = "hard"
```

#### Dependency Sources

##### Modrinth
```toml
mod_name = { modrinth = "PROJECT_ID", version = "VERSION" }
```
- **PROJECT_ID**: Modrinth project identifier (e.g., "P7dR8mSH" for Fabric API)
- **VERSION**: Exact version string or version range

##### CurseForge
```toml
mod_name = { curseforge = PROJECT_ID, version = "VERSION" }
```
- **PROJECT_ID**: Numeric CurseForge project ID
- **VERSION**: Exact version string

##### Direct URLs
```toml
mod_name = { url = "https://example.com/mod.jar", hash = "sha256:HASH" }
```
- **URL**: Direct download link
- **HASH**: SHA-256 hash for verification (format: "sha256:HEX")

#### Platform Filtering

Dependencies can be restricted to specific platforms:

```toml
[dependencies]
# Only on Linux
linux_only_mod = { modrinth = "ABC123", version = "1.0.0", platforms = ["linux"] }

# Not on Windows
not_windows = { modrinth = "DEF456", version = "1.0.0", platforms = ["!windows"] }

# Specific architectures
arm_mod = { modrinth = "GHI789", version = "1.0.0", platforms = ["linux/arm64"] }
```

Supported platforms:
- `windows`, `linux`, `macos`
- `x86_64`, `aarch64` (architectures)
- Combined: `linux/x86_64`, `macos/aarch64`

### Server Configuration Files

Standard Minecraft server files are included directly in the repository:

#### `server.properties`
```properties
# Standard Minecraft server properties
server-port=25565
max-players=20
difficulty=normal
level-name=world
```

#### `ops.json`
```json
[
  {
    "uuid": "550e8400-e29b-41d4-a716-446655440000",
    "name": "AdminPlayer",
    "level": 4,
    "bypassPlayerLimit": true
  }
]
```

#### Mod Configuration Files

Mod-specific configuration files go in the `config/` directory:
```
config/
├── jei/
│   └── common.toml
├── luckperms/
│   ├── config.yml
│   └── storage-settings.yml
└── ...
```

## Binary Distribution Formats

Atlas Runner uses Protocol Buffers for efficient, structured data serialization.

### Protocol Buffer Schema (`atlas.proto`)

```protobuf
syntax = "proto3";

package atlas.protocol;

message PackBlob {
  PackMetadata metadata = 1;
  Manifest manifest = 2;
  map<string, bytes> files = 3;
}

message PackMetadata {
  string pack_id = 1;
  string version = 2;
  string minecraft_version = 3;
  Loader loader = 4;
  string loader_version = 5;
  string name = 6;
  string description = 7;
  uint64 created_at = 8;
}

message Manifest {
  repeated Dependency dependencies = 1;
}

message Dependency {
  string url = 1;
  Hash hash = 2;
  PlatformFilter platform = 3;
  DependencyKind kind = 4;
  DependencySide side = 5;
  string pointer_path = 6;  // For local files
}

message Hash {
  HashAlgorithm algorithm = 1;
  string hex = 2;
}

message PlatformFilter {
  repeated Platform include = 1;
  repeated Platform exclude = 2;
}

enum HashAlgorithm {
  HASH_ALGORITHM_SHA1 = 0;
  HASH_ALGORITHM_SHA256 = 1;
  HASH_ALGORITHM_SHA512 = 2;
}

enum DependencyKind {
  DEPENDENCY_KIND_MOD = 0;
  DEPENDENCY_KIND_RESOURCE = 1;
  DEPENDENCY_KIND_CONFIG = 2;
}

enum DependencySide {
  DEPENDENCY_SIDE_BOTH = 0;
  DEPENDENCY_SIDE_CLIENT = 1;
  DEPENDENCY_SIDE_SERVER = 2;
}

enum Loader {
  LOADER_VANILLA = 0;
  LOADER_FORGE = 1;
  LOADER_FABRIC = 2;
  LOADER_NEOFORGE = 3;
}

enum Platform {
  PLATFORM_WINDOWS = 0;
  PLATFORM_LINUX = 1;
  PLATFORM_MACOS = 2;
  PLATFORM_X86_64 = 3;
  PLATFORM_AARCH64 = 4;
}
```

### Pack Blob Structure

The `.bin` file is a Zstd-compressed Protocol Buffer containing:

#### PackMetadata
- **pack_id**: Unique identifier from `pack.toml`
- **version**: Semantic version string
- **minecraft_version**: Target Minecraft version
- **loader**: Mod loader enumeration
- **loader_version**: Loader version (if applicable)
- **name**: Human-readable pack name
- **description**: Pack description
- **created_at**: Build timestamp (Unix epoch)

#### Manifest
List of external dependencies that must be downloaded:

```protobuf
message Dependency {
  url: "https://cdn.modrinth.com/data/P7dR8mSH/versions/0.92.0+1.20.1/fabric-api-0.92.0+1.20.1.jar"
  hash: {
    algorithm: HASH_ALGORITHM_SHA256
    hex: "a1b2c3d4..."
  }
  platform: {
    include: [PLATFORM_LINUX, PLATFORM_MACOS, PLATFORM_WINDOWS]
  }
  kind: DEPENDENCY_KIND_MOD
  side: DEPENDENCY_SIDE_BOTH
}
```

#### Files Map
Virtual filesystem mapping relative paths to byte content:

```protobuf
files: {
  "server.properties": <bytes of server.properties>
  "config/jei/common.toml": <bytes of jei config>
  "mods/fabric-api.jar": <bytes of local mod file>
}
```

### Compression and Encoding

#### Zstd Compression
- **Level**: 19 (high compression, slow)
- **Dictionary**: None (general-purpose compression)
- **Checksums**: Enabled for integrity

#### Binary Encoding
- **Protocol Buffers v3**: Efficient binary serialization
- **Varint Encoding**: Compact integer representation
- **Length-Delimited Fields**: Variable-length byte arrays

### Build Process

1. **Parse `pack.toml`**: Extract metadata and dependencies
2. **Resolve Dependencies**: Convert modrinth/curseforge IDs to URLs
3. **Download Artifacts**: Fetch and verify external dependencies
4. **Collect Files**: Read all repository files into memory
5. **Build Manifest**: Create dependency list with hashes
6. **Serialize**: Encode PackBlob Protocol Buffer
7. **Compress**: Apply Zstd compression
8. **Upload**: Send to Atlas Hub

### Runtime Application

1. **Download**: Fetch .bin file from Hub
2. **Decompress**: Zstd decompression
3. **Deserialize**: Parse Protocol Buffer
4. **Extract Files**: Write embedded files to server directory
5. **Download Dependencies**: Fetch external JARs with verification
6. **Platform Filter**: Apply OS/architecture rules
7. **Launch**: Generate JVM arguments and start server

## IPC Protocol Format

CLI-daemon communication uses Protocol Buffers over Unix sockets.

### Message Framing

```
[4-byte length][protobuf message]
```

- **Length**: Big-endian 32-bit unsigned integer
- **Message**: Protocol Buffer encoded request/response

### Request Messages

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

message Start {
  string profile = 1;
  map<string, string> env = 2;
}

message LogsTail {
  string profile = 1;
  uint32 lines = 2;
}

message RconExec {
  string profile = 1;
  string command = 2;
}
```

### Response Messages

```protobuf
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

message Status {
  string profile = 1;
  State state = 2;
  uint32 pid = 3;
  uint64 uptime_seconds = 4;
  string minecraft_version = 5;
  string pack_version = 6;
}

enum State {
  STOPPED = 0;
  STARTING = 1;
  RUNNING = 2;
  STOPPING = 3;
  ERROR = 4;
}
```

### Error Handling

```protobuf
message Error {
  string message = 1;
  string code = 2;
}
```

Common error codes:
- `INVALID_REQUEST`: Malformed message
- `SERVER_NOT_FOUND`: Profile doesn't exist
- `ALREADY_RUNNING`: Server already started
- `NOT_RUNNING`: Server not running
- `PERMISSION_DENIED`: Access denied
- `INTERNAL_ERROR`: Unexpected error

## Configuration Formats

### Deploy Configuration (`deploy.json`)

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

### Server State (`state.json`)

```json
{
  "currentBuildId": "abc123...",
  "lastUpdateCheck": 1640995200,
  "serverPid": 12345,
  "startTime": 1640995300
}
```

## Version Compatibility

### Format Versions
- **Protocol Buffers**: v3 (backward compatible)
- **TOML Schema**: v1.0 (semantic versioning)
- **IPC Protocol**: v2 (breaking changes allowed)

### Migration Strategy
- **Forward Compatibility**: New fields are optional
- **Schema Evolution**: Use Protocol Buffer field numbers carefully
- **Version Detection**: Metadata includes format version information

## Performance Considerations

### Compression Ratios
- **Text Files**: 70-90% compression ratio
- **Binary Files**: 10-30% compression ratio
- **Overall**: 50-80% size reduction

### Parsing Performance
- **Protocol Buffers**: Fast binary parsing (microseconds)
- **TOML**: Human-readable but slower parsing
- **Zstd**: Fast decompression with low memory usage

### Memory Usage
- **Blob Loading**: Entire blob decompressed into memory
- **Streaming**: Future optimization for large blobs
- **Caching**: Prevents redundant downloads/parsing