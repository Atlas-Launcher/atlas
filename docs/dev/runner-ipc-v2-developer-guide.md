# Runner IPC v2 Developer Documentation

The `runner-ipc-v2` crate implements the inter-process communication protocol between the Atlas Runner CLI and daemon processes using Protocol Buffers over Unix domain sockets.

## Architecture

### Dependencies
```toml
[dependencies]
prost = "0.12"                                    # Protocol Buffers
prost-types = "0.12"                              # Protobuf types
tokio = { version = "1.0", features = ["net", "io-util", "sync"] } # Async I/O
tokio-util = { version = "0.7", features = ["codec"] } # Stream codecs
futures = "0.3"                                    # Stream utilities
serde = { version = "1.0", features = ["derive"] } # Serialization
anyhow = "1.0"                                      # Error handling
thiserror = "1.0"                                   # Error types
bytes = "1.0"                                       # Byte buffers
```

### Build Dependencies
```toml
[build-dependencies]
prost-build = "0.12"                               # Protobuf code generation
```

### Module Structure
```
src/
├── lib.rs              # Library exports
├── codec.rs            # Message encoding/decoding
├── client.rs           # IPC client implementation
├── server.rs           # IPC server implementation
├── types.rs            # Message type definitions
└── error.rs            # Error types

proto/
├── ipc.proto           # IPC message definitions
└── common.proto        # Common types
```

## Core Components

### Protocol Buffers Definitions

#### IPC Messages
```protobuf
// proto/ipc.proto
syntax = "proto3";

package atlas.ipc;

import "common.proto";

message Request {
    uint64 id = 1;
    oneof payload {
        StartServerRequest start_server = 2;
        StopServerRequest stop_server = 3;
        GetStatusRequest get_status = 4;
        SendCommandRequest send_command = 5;
        GetLogsRequest get_logs = 6;
        CreateBackupRequest create_backup = 7;
        ListBackupsRequest list_backups = 8;
        RestoreBackupRequest restore_backup = 9;
        UpdateServerRequest update_server = 10;
        GetConfigRequest get_config = 11;
        SetConfigRequest set_config = 12;
    }
}

message Response {
    uint64 id = 1;
    oneof payload {
        StartServerResponse start_server = 2;
        StopServerResponse stop_server = 3;
        GetStatusResponse get_status = 4;
        SendCommandResponse send_command = 5;
        GetLogsResponse get_logs = 6;
        CreateBackupResponse create_backup = 7;
        ListBackupsResponse list_backups = 8;
        RestoreBackupResponse restore_backup = 9;
        UpdateServerResponse update_server = 10;
        GetConfigResponse get_config = 11;
        SetConfigResponse set_config = 12;
        ErrorResponse error = 13;
    }
}
```

#### Request/Response Types
```protobuf
// Server control messages
message StartServerRequest {
    string channel = 1;
    bool wait_for_ready = 2;
}

message StartServerResponse {
    bool success = 1;
    string server_pid = 2;
    string message = 3;
}

message StopServerRequest {
    bool force = 1;
    uint32 timeout_seconds = 2;
}

message StopServerResponse {
    bool success = 1;
    string message = 2;
}

// Status messages
message GetStatusRequest {}

message GetStatusResponse {
    ServerState state = 1;
    string version = 2;
    uint64 uptime_seconds = 3;
    uint64 memory_usage_mb = 4;
    uint32 player_count = 5;
    uint32 max_players = 6;
    string motd = 7;
}

// Command messages
message SendCommandRequest {
    string command = 1;
}

message SendCommandResponse {
    bool success = 1;
    string response = 2;
}

// Log messages
message GetLogsRequest {
    uint32 lines = 1;
    bool follow = 2;
}

message GetLogsResponse {
    repeated string lines = 1;
    bool eof = 2;
}

// Backup messages
message CreateBackupRequest {
    string name = 1;
}

message CreateBackupResponse {
    bool success = 1;
    string backup_id = 2;
    string message = 3;
}

message ListBackupsRequest {}

message ListBackupsResponse {
    repeated BackupInfo backups = 1;
}

message RestoreBackupRequest {
    string backup_id = 1;
}

message RestoreBackupResponse {
    bool success = 1;
    string message = 2;
}

// Update messages
message UpdateServerRequest {
    string channel = 1;
}

message UpdateServerResponse {
    bool success = 1;
    string old_version = 2;
    string new_version = 3;
    string message = 4;
}

// Configuration messages
message GetConfigRequest {}

message GetConfigResponse {
    string config_json = 1;
}

message SetConfigRequest {
    string config_json = 1;
}

message SetConfigResponse {
    bool success = 1;
    string message = 2;
}

// Error response
message ErrorResponse {
    string code = 1;
    string message = 2;
    string details = 3;
}
```

#### Common Types
```protobuf
// proto/common.proto
syntax = "proto3";

package atlas.common;

enum ServerState {
    SERVER_STATE_UNKNOWN = 0;
    SERVER_STATE_STOPPED = 1;
    SERVER_STATE_STARTING = 2;
    SERVER_STATE_RUNNING = 3;
    SERVER_STATE_STOPPING = 4;
    SERVER_STATE_ERROR = 5;
}

message BackupInfo {
    string id = 1;
    string name = 2;
    uint64 created_at = 3;
    uint64 size_bytes = 4;
}
```

### Build Script
```rust
// build.rs
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from("proto");

    prost_build::Config::new()
        .out_dir("src/")
        .compile_protos(&[
            proto_dir.join("ipc.proto"),
            proto_dir.join("common.proto"),
        ])?;

    Ok(())
}
```

### Message Codec

#### Length-Prefixed Codec
```rust
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
use bytes::{Buf, BufMut, BytesMut};
use prost::Message;

pub struct IpcCodec {
    inner: LengthDelimitedCodec,
}

impl IpcCodec {
    pub fn new() -> Self {
        let mut codec = LengthDelimitedCodec::new();
        codec.set_max_frame_length(10 * 1024 * 1024); // 10MB max message size
        Self { inner: codec }
    }
}

impl Decoder for IpcCodec {
    type Item = ipc::Response;
    type Error = IpcError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.inner.decode(src)? {
            Some(frame) => {
                let response = ipc::Response::decode(frame)?;
                Ok(Some(response))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<ipc::Request> for IpcCodec {
    type Error = IpcError;

    fn encode(&mut self, item: ipc::Request, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut buf = Vec::new();
        item.encode(&mut buf)?;
        self.inner.encode(Bytes::from(buf), dst)?;
        Ok(())
    }
}
```

#### Stream Wrapper
```rust
use tokio::net::UnixStream;
use tokio_util::codec::Framed;

pub type IpcStream = Framed<UnixStream, IpcCodec>;

pub fn wrap_stream(stream: UnixStream) -> IpcStream {
    Framed::new(stream, IpcCodec::new())
}
```

### IPC Client

#### Client Implementation
```rust
use tokio::sync::mpsc;
use std::collections::HashMap;

pub struct IpcClient {
    stream: IpcStream,
    next_id: u64,
    pending_requests: HashMap<u64, mpsc::Sender<Result<ipc::Response>>>,
}

impl IpcClient {
    pub async fn connect(socket_path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        let stream = wrap_stream(stream);

        Ok(Self {
            stream,
            next_id: 1,
            pending_requests: HashMap::new(),
        })
    }

    pub async fn send_request(&mut self, payload: impl Into<ipc::request::Payload>) -> Result<ipc::Response> {
        let id = self.next_id;
        self.next_id += 1;

        let request = ipc::Request {
            id,
            payload: Some(payload.into()),
        };

        // Create channel for response
        let (tx, mut rx) = mpsc::channel(1);
        self.pending_requests.insert(id, tx);

        // Send request
        self.stream.send(request).await?;

        // Wait for response
        match rx.recv().await {
            Some(result) => result,
            None => Err(IpcError::ConnectionClosed),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(response) = self.stream.next() => {
                    let response = response?;
                    if let Some(tx) = self.pending_requests.remove(&response.id) {
                        let _ = tx.send(Ok(response)).await;
                    }
                }
                else => break,
            }
        }
        Ok(())
    }
}
```

#### High-Level API
```rust
impl IpcClient {
    pub async fn start_server(&mut self, channel: &str, wait_for_ready: bool) -> Result<StartServerResponse> {
        let request = ipc::StartServerRequest {
            channel: channel.to_string(),
            wait_for_ready,
        };

        let response = self.send_request(request).await?;
        match response.payload {
            Some(ipc::response::Payload::StartServer(resp)) => Ok(resp),
            Some(ipc::response::Payload::Error(err)) => Err(IpcError::ServerError(err)),
            _ => Err(IpcError::UnexpectedResponse),
        }
    }

    pub async fn stop_server(&mut self, force: bool, timeout_seconds: u32) -> Result<StopServerResponse> {
        let request = ipc::StopServerRequest {
            force,
            timeout_seconds,
        };

        let response = self.send_request(request).await?;
        match response.payload {
            Some(ipc::response::Payload::StopServer(resp)) => Ok(resp),
            Some(ipc::response::Payload::Error(err)) => Err(IpcError::ServerError(err)),
            _ => Err(IpcError::UnexpectedResponse),
        }
    }

    pub async fn get_status(&mut self) -> Result<GetStatusResponse> {
        let request = ipc::GetStatusRequest {};

        let response = self.send_request(request).await?;
        match response.payload {
            Some(ipc::response::Payload::GetStatus(resp)) => Ok(resp),
            Some(ipc::response::Payload::Error(err)) => Err(IpcError::ServerError(err)),
            _ => Err(IpcError::UnexpectedResponse),
        }
    }

    pub async fn send_command(&mut self, command: &str) -> Result<SendCommandResponse> {
        let request = ipc::SendCommandRequest {
            command: command.to_string(),
        };

        let response = self.send_request(request).await?;
        match response.payload {
            Some(ipc::response::Payload::SendCommand(resp)) => Ok(resp),
            Some(ipc::response::Payload::Error(err)) => Err(IpcError::ServerError(err)),
            _ => Err(IpcError::UnexpectedResponse),
        }
    }
}
```

### IPC Server

#### Server Implementation
```rust
use tokio::net::UnixListener;
use tokio::sync::mpsc;
use std::sync::Arc;

pub struct IpcServer {
    listener: UnixListener,
    handler: Arc<dyn RequestHandler>,
}

#[async_trait::async_trait]
pub trait RequestHandler: Send + Sync {
    async fn handle_request(&self, request: ipc::Request) -> Result<ipc::Response>;
}

impl IpcServer {
    pub async fn bind(socket_path: impl AsRef<Path>, handler: Arc<dyn RequestHandler>) -> Result<Self> {
        let listener = UnixListener::bind(socket_path)?;
        Ok(Self { listener, handler })
    }

    pub async fn run(self) -> Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let handler = self.handler.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, handler).await {
                    eprintln!("Connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(stream: UnixStream, handler: Arc<dyn RequestHandler>) -> Result<()> {
    let mut stream = wrap_stream(stream);

    while let Some(request) = stream.next().await {
        let request = request?;
        let response = handler.handle_request(request).await?;
        stream.send(response).await?;
    }

    Ok(())
}
```

#### Request Handler Implementation
```rust
use std::sync::RwLock;

pub struct ServerHandler {
    state: Arc<RwLock<ServerState>>,
    supervisor: Arc<ServerSupervisor>,
}

#[async_trait::async_trait]
impl RequestHandler for ServerHandler {
    async fn handle_request(&self, request: ipc::Request) -> Result<ipc::Response> {
        let response = match request.payload {
            Some(ipc::request::Payload::StartServer(req)) => {
                self.handle_start_server(req).await?
            }
            Some(ipc::request::Payload::StopServer(req)) => {
                self.handle_stop_server(req).await?
            }
            Some(ipc::request::Payload::GetStatus(_)) => {
                self.handle_get_status().await?
            }
            Some(ipc::request::Payload::SendCommand(req)) => {
                self.handle_send_command(req).await?
            }
            // ... other handlers
            _ => ipc::Response {
                id: request.id,
                payload: Some(ipc::response::Payload::Error(ipc::ErrorResponse {
                    code: "UNKNOWN_REQUEST".to_string(),
                    message: "Unknown request type".to_string(),
                    details: String::new(),
                })),
            },
        };

        Ok(ipc::Response {
            id: request.id,
            payload: response.payload,
        })
    }
}
```

## Error Handling

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Unexpected response type")]
    UnexpectedResponse,

    #[error("Server error: {code} - {message}")]
    ServerError(ipc::ErrorResponse),

    #[error("Request timeout")]
    Timeout,

    #[error("Invalid socket path: {0}")]
    InvalidSocketPath(String),

    #[error("Message too large")]
    MessageTooLarge,
}
```

### Error Recovery
```rust
impl IpcClient {
    pub async fn send_request_with_retry(
        &mut self,
        payload: impl Into<ipc::request::Payload>,
        max_retries: u32,
    ) -> Result<ipc::Response> {
        let mut last_error = None;

        for attempt in 0..max_retries {
            match self.send_request(payload).await {
                Ok(response) => return Ok(response),
                Err(IpcError::Io(e)) if attempt < max_retries - 1 => {
                    // Retry on connection errors
                    if e.kind() == std::io::ErrorKind::ConnectionRefused ||
                       e.kind() == std::io::ErrorKind::BrokenPipe {
                        tokio::time::sleep(Duration::from_millis(100 * 2u64.pow(attempt))).await;
                        continue;
                    }
                }
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }
        }

        Err(last_error.unwrap_or(IpcError::Timeout))
    }
}
```

## Performance Considerations

### Connection Pooling
```rust
use tokio::sync::Semaphore;

pub struct PooledIpcClient {
    socket_path: PathBuf,
    max_connections: usize,
    semaphore: Semaphore,
}

impl PooledIpcClient {
    pub fn new(socket_path: PathBuf, max_connections: usize) -> Self {
        Self {
            socket_path,
            max_connections,
            semaphore: Semaphore::new(max_connections),
        }
    }

    pub async fn execute<F, T>(&self, operation: F) -> Result<T>
    where
        F: AsyncFnOnce(&mut IpcClient) -> Result<T>,
    {
        let _permit = self.semaphore.acquire().await.unwrap();
        let mut client = IpcClient::connect(&self.socket_path).await?;
        operation(&mut client).await
    }
}
```

### Message Batching
```rust
impl IpcClient {
    pub async fn send_batch(&mut self, requests: Vec<ipc::Request>) -> Result<Vec<ipc::Response>> {
        // Send all requests
        for request in &requests {
            self.stream.send(request.clone()).await?;
        }

        // Collect responses in order
        let mut responses = Vec::new();
        for _ in 0..requests.len() {
            let response = self.stream.next().await
                .ok_or(IpcError::ConnectionClosed)??;
            responses.push(response);
        }

        // Sort responses by ID to match request order
        responses.sort_by_key(|r| r.id);

        Ok(responses)
    }
}
```

### Streaming Responses
```rust
pub struct StreamingResponse<T> {
    receiver: mpsc::Receiver<Result<T>>,
}

impl<T> StreamingResponse<T> {
    pub async fn next(&mut self) -> Option<Result<T>> {
        self.receiver.recv().await
    }

    pub fn into_stream(self) -> impl Stream<Item = Result<T>> {
        tokio_stream::wrappers::ReceiverStream::new(self.receiver)
    }
}

impl IpcClient {
    pub async fn get_logs_streaming(&mut self, lines: u32) -> Result<StreamingResponse<String>> {
        let request = ipc::GetLogsRequest {
            lines,
            follow: true,
        };

        let (tx, rx) = mpsc::channel(100);

        // Send request
        self.stream.send(ipc::Request {
            id: self.next_id,
            payload: Some(ipc::request::Payload::GetLogs(request)),
        }).await?;
        self.next_id += 1;

        // Spawn task to handle streaming response
        tokio::spawn(async move {
            // Handle streaming log responses...
        });

        Ok(StreamingResponse { receiver: rx })
    }
}
```

## Security

### Socket Permissions
```rust
use std::os::unix::fs::PermissionsExt;

pub async fn secure_socket(socket_path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        // Set restrictive permissions (owner read/write only)
        let mut perms = tokio::fs::metadata(socket_path).await?.permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(socket_path, perms).await?;
    }
    Ok(())
}
```

### Message Validation
```rust
impl ipc::Request {
    pub fn validate(&self) -> Result<()> {
        match &self.payload {
            Some(ipc::request::Payload::SendCommand(req)) => {
                if req.command.is_empty() {
                    return Err(IpcError::Validation("Command cannot be empty".to_string()));
                }
                if req.command.len() > 1000 {
                    return Err(IpcError::Validation("Command too long".to_string()));
                }
                // Check for dangerous commands
                if req.command.contains("rm ") || req.command.contains("shutdown") {
                    return Err(IpcError::Validation("Dangerous command not allowed".to_string()));
                }
            }
            Some(ipc::request::Payload::SetConfig(req)) => {
                // Validate JSON structure
                serde_json::from_str::<serde_json::Value>(&req.config_json)
                    .map_err(|e| IpcError::Validation(format!("Invalid JSON: {}", e)))?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Rate Limiting
```rust
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct RateLimiter {
    limits: RwLock<HashMap<String, (u64, Instant)>>,
    max_requests_per_minute: u64,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u64) -> Self {
        Self {
            limits: RwLock::new(HashMap::new()),
            max_requests_per_minute,
        }
    }

    pub async fn check_rate_limit(&self, client_id: &str) -> Result<()> {
        let mut limits = self.limits.write().await;
        let now = Instant::now();

        let (count, window_start) = limits.entry(client_id.to_string())
            .or_insert((0, now));

        // Reset counter if window has passed
        if now.duration_since(*window_start) > Duration::from_secs(60) {
            *count = 0;
            *window_start = now;
        }

        if *count >= self.max_requests_per_minute {
            return Err(IpcError::RateLimited);
        }

        *count += 1;
        Ok(())
    }
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_message_encoding() {
        let request = ipc::Request {
            id: 123,
            payload: Some(ipc::request::Payload::GetStatus(ipc::GetStatusRequest {})),
        };

        let mut codec = IpcCodec::new();
        let mut buf = BytesMut::new();

        codec.encode(request.clone(), &mut buf).unwrap();
        let decoded = codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(decoded.id, request.id);
    }

    #[test]
    fn test_request_validation() {
        let valid_request = ipc::Request {
            id: 1,
            payload: Some(ipc::request::Payload::SendCommand(ipc::SendCommandRequest {
                command: "list".to_string(),
            })),
        };
        assert!(valid_request.validate().is_ok());

        let invalid_request = ipc::Request {
            id: 1,
            payload: Some(ipc::request::Payload::SendCommand(ipc::SendCommandRequest {
                command: "rm -rf /".to_string(),
            })),
        };
        assert!(invalid_request.validate().is_err());
    }
}
```

### Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    struct MockHandler;

    #[async_trait::async_trait]
    impl RequestHandler for MockHandler {
        async fn handle_request(&self, request: ipc::Request) -> Result<ipc::Response> {
            match request.payload {
                Some(ipc::request::Payload::GetStatus(_)) => {
                    Ok(ipc::Response {
                        id: request.id,
                        payload: Some(ipc::response::Payload::GetStatus(ipc::GetStatusResponse {
                            state: ipc::ServerState::Running as i32,
                            version: "1.0.0".to_string(),
                            uptime_seconds: 3600,
                            memory_usage_mb: 2048,
                            player_count: 5,
                            max_players: 20,
                            motd: "Test Server".to_string(),
                        })),
                    })
                }
                _ => Ok(ipc::Response {
                    id: request.id,
                    payload: Some(ipc::response::Payload::Error(ipc::ErrorResponse {
                        code: "NOT_IMPLEMENTED".to_string(),
                        message: "Mock handler".to_string(),
                        details: String::new(),
                    })),
                }),
            }
        }
    }

    #[tokio::test]
    async fn test_client_server_integration() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Start server
        let handler = Arc::new(MockHandler);
        let server = IpcServer::bind(&socket_path, handler).await.unwrap();
        let server_handle = tokio::spawn(async move {
            server.run().await
        });

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Connect client
        let mut client = IpcClient::connect(&socket_path).await.unwrap();

        // Test request
        let status = client.get_status().await.unwrap();
        assert_eq!(status.state, ipc::ServerState::Running as i32);
        assert_eq!(status.version, "1.0.0");

        server_handle.abort();
    }
}
```

### Load Tests
```rust
#[cfg(test)]
mod load_tests {
    use super::*;
    use tokio::sync::Barrier;

    #[tokio::test]
    async fn test_concurrent_requests() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Start server
        let handler = Arc::new(MockHandler);
        let server = IpcServer::bind(&socket_path, handler).await.unwrap();
        let server_handle = tokio::spawn(async move { server.run().await });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let barrier = Arc::new(Barrier::new(10));
        let mut tasks = Vec::new();

        for _ in 0..10 {
            let barrier = barrier.clone();
            let socket_path = socket_path.clone();

            let task = tokio::spawn(async move {
                barrier.wait().await;
                let mut client = IpcClient::connect(&socket_path).await.unwrap();

                for _ in 0..100 {
                    let _ = client.get_status().await.unwrap();
                }
            });

            tasks.push(task);
        }

        for task in tasks {
            task.await.unwrap();
        }

        server_handle.abort();
    }
}
```

## Usage Examples

### Client Usage
```rust
use runner_ipc_v2::IpcClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Connect to daemon
    let mut client = IpcClient::connect("/tmp/atlas-runner.sock").await?;

    // Start server
    let response = client.start_server("latest", true).await?;
    println!("Server started: {}", response.message);

    // Get status
    let status = client.get_status().await?;
    println!("Server state: {:?}", status.state);
    println!("Players: {}/{}", status.player_count, status.max_players);

    // Send command
    let cmd_response = client.send_command("list").await?;
    println!("Command response: {}", cmd_response.response);

    // Stop server
    let stop_response = client.stop_server(false, 30).await?;
    println!("Server stopped: {}", stop_response.message);

    Ok(())
}
```

### Server Implementation
```rust
use runner_ipc_v2::{IpcServer, RequestHandler};
use std::sync::Arc;

struct MyHandler {
    // Server state...
}

#[async_trait::async_trait]
impl RequestHandler for MyHandler {
    async fn handle_request(&self, request: ipc::Request) -> Result<ipc::Response> {
        match request.payload {
            Some(ipc::request::Payload::StartServer(req)) => {
                // Start the server...
                Ok(ipc::Response {
                    id: request.id,
                    payload: Some(ipc::response::Payload::StartServer(
                        ipc::StartServerResponse {
                            success: true,
                            server_pid: "1234".to_string(),
                            message: "Server started successfully".to_string(),
                        }
                    )),
                })
            }
            Some(ipc::request::Payload::GetStatus(_)) => {
                // Get server status...
                Ok(ipc::Response {
                    id: request.id,
                    payload: Some(ipc::response::Payload::GetStatus(
                        ipc::GetStatusResponse {
                            state: ipc::ServerState::Running as i32,
                            version: "1.20.1".to_string(),
                            uptime_seconds: 3600,
                            memory_usage_mb: 4096,
                            player_count: 3,
                            max_players: 20,
                            motd: "Welcome!".to_string(),
                        }
                    )),
                })
            }
            _ => Ok(ipc::Response {
                id: request.id,
                payload: Some(ipc::response::Payload::Error(
                    ipc::ErrorResponse {
                        code: "NOT_IMPLEMENTED".to_string(),
                        message: "Command not implemented".to_string(),
                        details: String::new(),
                    }
                )),
            }),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let handler = Arc::new(MyHandler {});
    let server = IpcServer::bind("/tmp/atlas-runner.sock", handler).await?;
    server.run().await?;
    Ok(())
}
```

### Streaming Logs
```rust
use runner_ipc_v2::StreamingResponse;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = IpcClient::connect("/tmp/atlas-runner.sock").await?;

    // Start streaming logs
    let mut log_stream = client.get_logs_streaming(100).await?;

    // Process log lines
    while let Some(line_result) = log_stream.next().await {
        match line_result {
            Ok(line) => println!("Log: {}", line),
            Err(e) => eprintln!("Error receiving log: {}", e),
        }
    }

    Ok(())
}
```

## Maintenance

### Protocol Evolution
```rust
// Version negotiation
impl ipc::Request {
    pub fn protocol_version(&self) -> u32 {
        // Extract version from message or use default
        1
    }
}

impl ipc::Response {
    pub fn protocol_version(&self) -> u32 {
        1
    }
}

// Backward compatibility handling
impl From<ipc::ResponseV1> for ipc::Response {
    fn from(v1: ipc::ResponseV1) -> Self {
        // Convert old format to new
        match v1.payload {
            // ... conversion logic
        }
    }
}
```

### Monitoring and Metrics
```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct IpcMetrics {
    pub connections_accepted: AtomicU64,
    pub requests_processed: AtomicU64,
    pub responses_sent: AtomicU64,
    pub errors_sent: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
}

impl IpcMetrics {
    pub fn record_connection(&self) {
        self.connections_accepted.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_request(&self, bytes: u64) {
        self.requests_processed.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_response(&self, bytes: u64, is_error: bool) {
        self.responses_sent.fetch_add(1, Ordering::Relaxed);
        if is_error {
            self.errors_sent.fetch_add(1, Ordering::Relaxed);
        }
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }
}
```

### Debugging Tools
```rust
impl IpcClient {
    pub async fn ping(&mut self) -> Result<Duration> {
        let start = Instant::now();
        let response = self.send_request(ipc::GetStatusRequest {}).await?;
        let duration = start.elapsed();

        match response.payload {
            Some(ipc::response::Payload::GetStatus(_)) => Ok(duration),
            _ => Err(IpcError::UnexpectedResponse),
        }
    }

    pub async fn debug_info(&mut self) -> Result<DebugInfo> {
        // Send debug request
        Ok(DebugInfo {
            ping_time: self.ping().await?,
            server_version: "1.0.0".to_string(),
            protocol_version: 1,
        })
    }
}

#[derive(Debug)]
pub struct DebugInfo {
    pub ping_time: Duration,
    pub server_version: String,
    pub protocol_version: u32,
}
```