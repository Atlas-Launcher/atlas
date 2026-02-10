# Atlas Runner API Reference

## IPC Protocol

Atlas Runner uses a custom Protocol Buffer-based IPC protocol over Unix domain sockets for communication between the CLI and daemon.

### Connection Details

- **Socket Path**: `~/.atlas/runnerd/sockets/daemon.sock`
- **Framing**: Length-prefixed messages (4-byte big-endian length + protobuf payload)
- **Encoding**: Protocol Buffers v3

### Message Flow

```
CLI → Daemon: Request
Daemon → CLI: Response
```

### Request Types

#### Ping
Check daemon connectivity.

**Request**:
```protobuf
message Ping {}
```

**Response**:
```protobuf
message Pong {}
```

#### Start Server
Start a Minecraft server instance.

**Request**:
```protobuf
message Start {
  string profile = 1;  // Server profile name (default: "default")
  map<string, string> env = 2;  // Environment variables
}
```

**Response**:
```protobuf
message Started {
  string profile = 1;
  uint32 pid = 2;  // Process ID
}
```

#### Stop Server
Stop a running Minecraft server.

**Request**:
```protobuf
message Stop {
  string profile = 1;
  bool force = 2;  // Force kill if graceful shutdown fails
}
```

**Response**:
```protobuf
message Stopped {
  string profile = 1;
}
```

#### Get Status
Retrieve server status information.

**Request**:
```protobuf
message Status {
  string profile = 1;
}
```

**Response**:
```protobuf
message Status {
  string profile = 1;
  enum State {
    STOPPED = 0;
    STARTING = 1;
    RUNNING = 2;
    STOPPING = 3;
    ERROR = 4;
  }
  State state = 2;
  uint32 pid = 3;
  uint64 uptime_seconds = 4;
  string minecraft_version = 5;
  string pack_version = 6;
}
```

#### Tail Logs
Stream server log output.

**Request**:
```protobuf
message LogsTail {
  string profile = 1;
  uint32 lines = 2;  // Number of recent lines to return (0 = follow)
}
```

**Response**:
```protobuf
message LogsTail {
  repeated LogLine lines = 1;
}

message LogLine {
  uint64 timestamp = 1;  // Unix timestamp
  string level = 2;      // Log level (INFO, WARN, ERROR)
  string message = 3;    // Log message
}
```

#### Execute RCON Command
Execute a command via RCON.

**Request**:
```protobuf
message RconExec {
  string profile = 1;
  string command = 2;
}
```

**Response**:
```protobuf
message RconResult {
  string profile = 1;
  string result = 2;  // Command output
}
```

#### Save Deploy Key
Update daemon configuration.

**Request**:
```protobuf
message SaveDeployKey {
  string hub_url = 1;
  string pack_id = 2;
  string channel = 3;
  string deploy_token = 4;
  uint64 max_ram_mb = 5;
  bool should_autostart = 6;
  bool eula_accepted = 7;
}
```

**Response**:
```protobuf
message DeployKeySaved {}
```

### Error Handling

All requests can return an error response:

```protobuf
message Error {
  string message = 1;
  string code = 2;  // Error code for programmatic handling
}
```

**Common Error Codes**:
- `INVALID_REQUEST`: Malformed request
- `SERVER_NOT_FOUND`: Profile doesn't exist
- `ALREADY_RUNNING`: Server already started
- `NOT_RUNNING`: Server not started
- `PERMISSION_DENIED`: Insufficient permissions
- `INTERNAL_ERROR`: Unexpected daemon error

## Hub API

Atlas Hub provides RESTful APIs for pack management and distribution.

### Authentication
All requests require a deploy token in the `Authorization` header:
```
Authorization: Bearer <deploy_token>
```

### Endpoints

#### Get Latest Build
Retrieve metadata for the latest build in a channel.

```
GET /api/v1/packs/{packId}/{channel}/latest
```

**Response**:
```json
{
  "buildId": "abc123...",
  "packId": "my-pack",
  "channel": "production",
  "version": "1.2.3",
  "minecraftVersion": "1.20.1",
  "loader": "fabric",
  "createdAt": 1640995200,
  "dependencies": [
    {
      "url": "https://...",
      "hash": "sha256:...",
      "size": 12345
    }
  ]
}
```

#### Download Pack Blob
Download the compressed pack distribution blob.

```
GET /api/v1/packs/{packId}/builds/{buildId}/blob
```

**Response**: Binary Zstd-compressed Protocol Buffer blob

#### Get Whitelist
Retrieve the current player whitelist for a pack.

```
GET /api/v1/packs/{packId}/whitelist
```

**Response**:
```json
{
  "players": [
    {
      "uuid": "550e8400-e29b-41d4-a716-446655440000",
      "username": "player1"
    }
  ]
}
```

### Error Responses

All endpoints return standard HTTP status codes with JSON error bodies:

```json
{
  "error": "Pack not found",
  "code": "PACK_NOT_FOUND"
}
```

## Configuration Schema

### Deploy Configuration
Stored in `~/.atlas/runnerd/deploy.json`

```typescript
interface DeployConfig {
  hubUrl: string;           // Atlas Hub base URL
  packId: string;           // Pack identifier
  channel: "dev" | "beta" | "production";
  deployToken: string;      // Authentication token
  maxRamMb: number;         // Maximum RAM in MB
  shouldAutostart: boolean; // Auto-start on daemon launch
  eulaAccepted: boolean;    // EULA acceptance flag
}
```

### Server Runtime Configuration
Generated from pack metadata and user settings.

```typescript
interface ServerConfig {
  minecraftVersion: string;
  loader: string;           // "vanilla", "fabric", "forge", etc.
  javaArgs: string[];       // JVM arguments
  serverArgs: string[];     // Server-specific arguments
  dependencies: Dependency[];
  platformFilters: Filter[];
}

interface Dependency {
  url: string;
  hash: string;             // SHA256 hash
  size: number;             // Expected size in bytes
}

interface Filter {
  platform: "windows" | "macos" | "linux";
  arch: "x86_64" | "aarch64";
}
```

## Protocol Buffer Definitions

### Core Protocol
```protobuf
syntax = "proto3";

package atlas.runner.ipc.v2;

// Requests
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

// Responses
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

// Message definitions...
```

### Distribution Format
```protobuf
syntax = "proto3";

package atlas.protocol;

message DistributionBlob {
  Metadata metadata = 1;
  bytes payload = 2;
}

message Metadata {
  string pack_id = 1;
  string version = 2;
  string minecraft_version = 3;
  string loader = 4;
  repeated Dependency dependencies = 5;
  repeated Filter platform_filters = 6;
  uint64 created_at = 7;
}

message Dependency {
  string url = 1;
  string hash = 2;
  uint64 size = 3;
}

message Filter {
  string platform = 1;
  string arch = 2;
}
```

## Error Codes

### IPC Error Codes
- `INVALID_REQUEST`: Request payload is malformed
- `SERVER_NOT_FOUND`: Specified server profile doesn't exist
- `ALREADY_RUNNING`: Server is already running
- `NOT_RUNNING`: Server is not running
- `PERMISSION_DENIED`: Operation not allowed
- `TIMEOUT`: Operation timed out
- `INTERNAL_ERROR`: Unexpected internal error

### HTTP Error Codes
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Missing or invalid token
- `403 Forbidden`: Token lacks required permissions
- `404 Not Found`: Resource doesn't exist
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error

## Rate Limits

### Hub API
- Pack metadata: 100 requests/minute
- Blob downloads: 10 concurrent downloads
- Whitelist: 100 requests/minute

### IPC
- No rate limiting (local communication)
- Requests processed sequentially per client