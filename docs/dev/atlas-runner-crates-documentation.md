# Atlas Runner Crates Documentation

This document provides technical details for each Rust crate in the Atlas Runner workspace.

## Core Architecture

The Atlas Runner system is built as a modular Rust workspace with the following crate organization:

- **Application Crates**: End-user binaries (`runner-v2`, `runnerd-v2`)
- **Library Crates**: Reusable components (`atlas-client`, `protocol`, etc.)
- **Internal Crates**: Implementation details (`runner-core-v2`, `runner-ipc-v2`)

## Application Crates

### `runner-v2` - CLI Application

**Path**: `apps/runner-v2/`
**Purpose**: Command-line interface for Atlas Runner
**Dependencies**: `atlas-client`, `runner-core-v2`, `runner-ipc-v2`, `tokio`

#### Key Components

##### Command Line Interface
- **clap**: Argument parsing and subcommand handling
- **Commands**: `up`, `down`, `status`, `logs`, `exec`, `systemd`
- **Interactive Prompts**: First-run setup with OAuth, channel selection, RAM config

##### Daemon Communication
- **IPC Client**: Connects to daemon via Unix socket
- **Protocol Buffers**: Encodes/decodes messages using `prost`
- **Async Operations**: Uses Tokio for non-blocking I/O

##### Configuration Management
- **Deploy Config**: Loads/saves `~/.atlas/runnerd/deploy.json`
- **First-run Setup**: Guides users through authentication and preferences
- **Persistent Settings**: RAM limits, channel selection, EULA acceptance

#### Example Usage
```rust
use clap::Parser;
use runner_v2::commands::core::up_command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::Up => up_command().await?,
        Commands::Status => status_command().await?,
        // ...
    }
    Ok(())
}
```

### `runnerd-v2` - Daemon Service

**Path**: `apps/runnerd-v2/`
**Purpose**: Background service managing Minecraft server lifecycle
**Dependencies**: `runner-core-v2`, `runner-ipc-v2`, `runner-provision-v2`, `tokio`

#### Key Components

##### Supervisor Module
- **Server Lifecycle**: Start/stop/restart Minecraft processes
- **Process Monitoring**: Health checks and crash recovery
- **Log Aggregation**: Captures stdout/stderr from server JVM

##### IPC Handler
- **Unix Socket Server**: Accepts connections from CLI
- **Message Processing**: Handles requests asynchronously
- **State Management**: Thread-safe access to server state

##### Update System
- **Polling**: Periodic checks for pack updates
- **Caching**: Build ID and blob caching for performance
- **Safe Updates**: Atomic server restarts with rollback

#### Architecture
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config()?;
    let state = Arc::new(Mutex::new(ServerState::default()));

    // Start IPC server
    let ipc_handle = tokio::spawn(ipc_server(state.clone()));

    // Start update poller
    let update_handle = tokio::spawn(update_poller(state.clone()));

    // Wait for shutdown signal
    shutdown_signal().await?;
    Ok(())
}
```

## Library Crates

### `atlas-client` - Hub API Client

**Path**: `crates/atlas-client/`
**Purpose**: HTTP client for Atlas Hub API communication
**Dependencies**: `reqwest`, `serde`, `tokio`

#### Key Features

##### OAuth Authentication
- **Device Code Flow**: Interactive authentication for CLI users
- **Token Management**: Automatic refresh of access tokens
- **Service Tokens**: Deploy token authentication for CI/builds

##### API Endpoints
- **Pack Metadata**: Retrieve pack information and builds
- **Blob Downloads**: Stream Protocol Buffer artifacts
- **Channel Management**: Get latest builds for channels
- **Whitelist Sync**: Player access control

#### Example Usage
```rust
use atlas_client::{HubClient, DeviceCodeFlow};

let client = HubClient::new(hub_url)?;
let tokens = client.authenticate_device_code().await?;
let pack_info = client.get_pack_metadata(pack_id).await?;
```

### `protocol` - Protocol Buffer Definitions

**Path**: `crates/protocol/`
**Purpose**: Serialization/deserialization for Atlas distribution format
**Dependencies**: `prost`, `zstd`, `serde`

#### Key Components

##### Pack Blob Format
```protobuf
message PackBlob {
  PackMetadata metadata = 1;
  Manifest manifest = 2;
  map<string, bytes> files = 3;
}
```

##### Build System
- **prost-build**: Code generation from `.proto` files
- **Zstd Compression**: Efficient blob compression/decompression
- **TOML Integration**: Creator-friendly configuration parsing

#### Usage
```rust
use protocol::{PackBlob, Compression};

// Decompress and parse blob
let decompressed = Compression::decompress(blob_bytes)?;
let pack: PackBlob = prost::Message::decode(&*decompressed)?;
```

### `mod-resolver` - Dependency Resolution

**Path**: `crates/mod-resolver/`
**Purpose**: Download and verify external mod dependencies
**Dependencies**: `reqwest`, `sha2`, `tokio`

#### Key Features

##### Multi-Source Support
- **Modrinth API**: Primary mod hosting platform
- **CurseForge API**: Alternative mod distribution
- **Custom URLs**: Direct artifact hosting
- **Mojang Servers**: Official Minecraft assets

##### Verification System
- **SHA-256 Hashes**: Cryptographic integrity checking
- **Platform Filtering**: OS/architecture-specific downloads
- **Caching**: Local artifact storage with validation

#### Example
```rust
use mod_resolver::{Resolver, Dependency};

let resolver = Resolver::new(cache_dir);
let artifact = resolver.resolve(&dependency).await?;
assert_eq!(artifact.hash, expected_sha256);
```

### `runner-core-v2` - Shared Types

**Path**: `crates/runner-core-v2/`
**Purpose**: Common data structures and utilities
**Dependencies**: `serde`, `thiserror`

#### Key Components

##### Configuration Types
```rust
#[derive(Serialize, Deserialize)]
pub struct DeployConfig {
    pub hub_url: String,
    pub pack_id: String,
    pub channel: String,
    pub deploy_token: String,
    pub max_ram_mb: u64,
    pub should_autostart: bool,
    pub eula_accepted: bool,
}
```

##### Error Handling
- **Custom Error Types**: Domain-specific error enumeration
- **Error Context**: Rich error messages with context
- **Result Types**: Type-safe error propagation

### `runner-ipc-v2` - Inter-Process Communication

**Path**: `crates/runner-ipc-v2/`
**Purpose**: Protocol for CLI-daemon communication
**Dependencies**: `tokio`, `tokio-util`, `prost`

#### Key Features

##### Message Protocol
- **Length-Prefixed Framing**: Reliable message boundaries
- **Protocol Buffers**: Efficient binary serialization
- **Async Streams**: Non-blocking I/O with Tokio

##### Message Types
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
```

##### Transport Layer
- **Unix Domain Sockets**: Local, secure communication
- **Connection Pooling**: Efficient resource usage
- **Error Recovery**: Automatic reconnection handling

### `runner-provision-v2` - Server Provisioning

**Path**: `crates/runner-provision-v2/`
**Purpose**: Apply pack blobs to server directories
**Dependencies**: `protocol`, `mod-resolver`, `tokio`

#### Key Components

##### Provisioning Process
1. **Blob Application**: Extract files from Protocol Buffer
2. **Dependency Resolution**: Download and verify mods
3. **Platform Filtering**: Apply OS/architecture rules
4. **JVM Preparation**: Generate launch arguments

##### Safety Features
- **Atomic Operations**: All-or-nothing directory updates
- **Backup Creation**: Preserve existing configurations
- **Validation**: Pre-flight checks before application

#### Example
```rust
use runner_provision_v2::Provisioner;

let provisioner = Provisioner::new(cache_dir);
provisioner.apply_pack_blob(&blob, &server_dir).await?;
```

### `runner-v2-rcon` - RCON Client

**Path**: `crates/runner-v2-rcon/`
**Purpose**: Remote console access to Minecraft servers
**Dependencies**: `tokio`, `minecraft-client-rs`

#### Key Features

##### Protocol Support
- **RCON Protocol**: Standard Minecraft remote console
- **Authentication**: Password-based security
- **Command Execution**: Synchronous and asynchronous commands

##### Interactive Mode
- **Shell Interface**: Full terminal experience
- **Command History**: Persistent command history
- **Auto-completion**: Context-aware suggestions

#### Usage
```rust
use runner_v2_rcon::RconClient;

let mut client = RconClient::connect(addr, password).await?;
let response = client.execute("list").await?;
```

### `runner-v2-utils` - Common Utilities

**Path**: `crates/runner-v2-utils/`
**Purpose**: Shared utility functions and helpers
**Dependencies**: Minimal dependencies for broad compatibility

#### Key Components

##### System Utilities
- **Path Management**: Cross-platform path operations
- **Process Tools**: System process interrogation
- **File Operations**: Safe file manipulation with error handling

##### Common Patterns
- **Result Extensions**: Convenience methods for Result types
- **Option Helpers**: Utility functions for Option types
- **String Processing**: Text manipulation and formatting

## Build System

### Cargo Workspace Configuration

**File**: `Cargo.toml` (root)
```toml
[workspace]
members = [
    "apps/*",
    "crates/*",
]
```

### Cross-Crate Dependencies
- **Version Alignment**: Consistent versioning across crates
- **Feature Flags**: Optional functionality (blocking, etc.)
- **Path Dependencies**: Local crate references for development

### Build Optimization
- **Release Profile**: Optimized builds for production
- **Debugging**: Development-friendly debug builds
- **Cross-Compilation**: Support for multiple target architectures

## Testing Strategy

### Unit Tests
- **Per-Crate**: Individual crate functionality
- **Mock Dependencies**: Isolated testing with mock implementations
- **Property Testing**: Generate test cases with proptest

### Integration Tests
- **IPC Testing**: CLI-daemon communication
- **End-to-End**: Full server deployment workflows
- **Network Simulation**: Mock Hub API responses

### CI/CD Integration
- **GitHub Actions**: Automated testing on pull requests
- **Cross-Platform**: Linux, macOS, Windows testing
- **Performance Benchmarks**: Regression detection

## Performance Characteristics

### Memory Usage
- **CLI**: ~10MB baseline, minimal runtime overhead
- **Daemon**: ~50MB baseline + server memory
- **Caching**: Significant memory savings through artifact reuse

### CPU Usage
- **Idle**: <1% CPU for monitoring/polling
- **Active**: Burst usage during updates/downloads
- **Optimization**: Async I/O prevents blocking operations

### Disk Usage
- **Cache**: Variable, depends on pack complexity
- **Logs**: Rotated, configurable retention
- **Backups**: Optional, user-configurable

## Security Considerations

### Authentication
- **OAuth 2.0**: Secure token-based authentication
- **Deploy Tokens**: High-entropy secrets for CI
- **No Credential Storage**: Tokens encrypted at rest

### Network Security
- **HTTPS Only**: All external communications encrypted
- **Certificate Validation**: Standard TLS verification
- **RCON Security**: Password-protected console access

### Process Isolation
- **User Permissions**: Runs as regular user, not root
- **File Permissions**: Restrictive access to configuration
- **Sandboxing**: JVM isolation for server processes

## Development Workflow

### Local Development
```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Check specific crate
cargo check -p atlas-client
```

### Debugging
- **Logging**: Structured logging with configurable levels
- **IPC Inspection**: Debug tools for message tracing
- **Performance Profiling**: Flame graphs and tracing

### Contributing
- **Code Standards**: Follow Rust idioms and conventions
- **Documentation**: Update docs for API changes
- **Testing**: Add tests for new functionality
- **Reviews**: Required for all changes