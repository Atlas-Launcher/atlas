# Atlas Runner API Reference

## IPC Protocol

Atlas Runner uses a JSON-based IPC protocol over Unix domain sockets for communication between the CLI and daemon.

### Connection Details

- **Socket Path**: `~/.atlas/runnerd/sockets/daemon.sock`
- **Framing**: Length-prefixed messages (4-byte big-endian length + JSON payload)
- **Encoding**: JSON with serde serialization

### Message Flow

```
CLI → Daemon: Request
Daemon → CLI: Response
```

### Request Types

#### Ping
Check daemon connectivity.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Ping",
    "data": {
      "client_version": "1.0.0",
      "protocol_version": 1
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Pong"
  }
}
```

#### Start Server
Start a Minecraft server instance.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Start",
    "data": {
      "profile": "default",
      "env": {
        "CUSTOM_VAR": "value"
      }
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Started",
    "data": {
      "profile": "default",
      "pid": 12345
    }
  }
}
```
}
```

#### Stop Server
Stop a running Minecraft server.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Stop",
    "data": {
      "profile": "default",
      "force": false
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Stopped",
    "data": {
      "profile": "default"
    }
  }
}
```

#### Get Status
Retrieve server status information.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Status",
    "data": {
      "profile": "default"
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "Status",
    "data": {
      "profile": "default",
      "state": "RUNNING",
      "pid": 12345,
      "uptime_seconds": 3600,
      "minecraft_version": "1.20.1",
      "pack_version": "1.2.3"
    }
  }
}
```

#### Tail Logs
Stream server log output.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "LogsTail",
    "data": {
      "profile": "default",
      "lines": 50
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "LogsTail",
    "data": {
      "lines": [
        {
          "timestamp": 1640995200,
          "level": "INFO",
          "message": "Server started successfully"
        },
        {
          "timestamp": 1640995260,
          "level": "WARN",
          "message": "Player joined: Steve"
        }
      ]
    }
  }
}
```

#### Execute RCON Command
Execute a command via RCON.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "RconExec",
    "data": {
      "profile": "default",
      "command": "say Hello World!"
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "RconResult",
    "data": {
      "profile": "default",
      "result": "Said 'Hello World!' to all players"
    }
  }
}
```

#### Save Deploy Key
Update daemon configuration.

**Request**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "SaveDeployKey",
    "data": {
      "hub_url": "https://hub.atlaslauncher.com",
      "pack_id": "my-pack",
      "channel": "production",
      "deploy_token": "abc123...",
      "max_ram_mb": 4096,
      "should_autostart": true,
      "eula_accepted": true
    }
  }
}
```

**Response**:
```json
{
  "id": "request-id",
  "payload": {
    "type": "DeployKeySaved",
    "data": {}
  }
}
```

### Error Handling

All requests can return an error response:

```json
{
  "id": "request-id",
  "error": {
    "message": "Server not found",
    "code": "SERVER_NOT_FOUND"
  }
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