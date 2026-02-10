# Runner Core v2 Developer Documentation

The `runner-core-v2` crate provides shared types, error handling, and core utilities used across the Atlas Runner ecosystem.

## Architecture

### Dependencies
```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }     # Serialization
thiserror = "1.0"                                       # Error types
anyhow = "1.0"                                          # Error handling
tokio = { version = "1.0", features = ["sync"] }        # Async utilities
uuid = { version = "1.0", features = ["v4"] }           # UUID generation
chrono = { version = "0.4", features = ["serde"] }      # Date/time handling
regex = "1.0"                                           # Regular expressions
url = "2.4"                                             # URL parsing
```

### Module Structure
```
src/
├── lib.rs              # Library exports
├── types.rs            # Core type definitions
├── error.rs            # Error types and handling
├── config.rs           # Configuration structures
├── validation.rs       # Input validation
├── logging.rs          # Logging utilities
└── utils.rs            # General utilities
```

## Core Components

### Type Definitions

#### Pack and Build Types
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub website_url: Option<String>,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Build {
    pub id: String,
    pub pack_id: String,
    pub version: String,
    pub changelog: Option<String>,
    pub minecraft_version: String,
    pub loader_version: String,
    pub java_version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ModLoader {
    #[serde(rename = "vanilla")]
    Vanilla,
    #[serde(rename = "fabric")]
    Fabric,
    #[serde(rename = "forge")]
    Forge,
    #[serde(rename = "neoforge")]
    NeoForge,
    #[serde(rename = "quilt")]
    Quilt,
}

impl ModLoader {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModLoader::Vanilla => "vanilla",
            ModLoader::Fabric => "fabric",
            ModLoader::Forge => "forge",
            ModLoader::NeoForge => "neoforge",
            ModLoader::Quilt => "quilt",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vanilla" => Some(ModLoader::Vanilla),
            "fabric" => Some(ModLoader::Fabric),
            "forge" => Some(ModLoader::Forge),
            "neoforge" => Some(ModLoader::NeoForge),
            "quilt" => Some(ModLoader::Quilt),
            _ => None,
        }
    }
}
```

#### Server Configuration
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub pack_id: String,
    pub channel: String,
    pub max_memory_mb: u64,
    pub min_memory_mb: u64,
    pub java_args: Vec<String>,
    pub server_args: Vec<String>,
    pub world_name: Option<String>,
    pub motd: Option<String>,
    pub max_players: Option<u32>,
    pub difficulty: Option<Difficulty>,
    pub gamemode: Option<GameMode>,
    pub pvp_enabled: Option<bool>,
    pub spawn_protection: Option<u32>,
    pub view_distance: Option<u32>,
    pub simulation_distance: Option<u32>,
    pub enable_whitelist: Option<bool>,
    pub enable_rcon: Option<bool>,
    pub rcon_password: Option<String>,
    pub rcon_port: Option<u16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            pack_id: String::new(),
            channel: "latest".to_string(),
            max_memory_mb: 4096,
            min_memory_mb: 1024,
            java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:+AlwaysPreTouch".to_string(),
            ],
            server_args: vec!["nogui".to_string()],
            world_name: None,
            motd: Some("Atlas Runner Server".to_string()),
            max_players: Some(20),
            difficulty: Some(Difficulty::Normal),
            gamemode: Some(GameMode::Survival),
            pvp_enabled: Some(true),
            spawn_protection: Some(16),
            view_distance: Some(10),
            simulation_distance: Some(10),
            enable_whitelist: Some(false),
            enable_rcon: Some(true),
            rcon_password: None,
            rcon_port: Some(25575),
        }
    }
}
```

#### Deployment Configuration
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeployConfig {
    pub hub_url: String,
    pub pack_id: String,
    pub channel: String,
    pub deploy_token: String,
    pub server_config: ServerConfig,
    pub auto_start: bool,
    pub auto_update: bool,
    pub backup_enabled: bool,
    pub backup_interval_hours: u32,
    pub max_backups: usize,
    pub eula_accepted: bool,
}

impl DeployConfig {
    pub fn validate(&self) -> Result<()> {
        if self.hub_url.is_empty() {
            return Err(Error::Validation("Hub URL cannot be empty".to_string()));
        }

        if self.pack_id.is_empty() {
            return Err(Error::Validation("Pack ID cannot be empty".to_string()));
        }

        if self.deploy_token.is_empty() {
            return Err(Error::Validation("Deploy token cannot be empty".to_string()));
        }

        if !self.eula_accepted {
            return Err(Error::Validation("EULA must be accepted".to_string()));
        }

        Ok(())
    }
}
```

### Error Handling

#### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("Pack error: {0}")]
    Pack(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Java error: {0}")]
    Java(String),

    #[error("RCON error: {0}")]
    Rcon(String),

    #[error("Backup error: {0}")]
    Backup(String),

    #[error("Update error: {0}")]
    Update(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}
```

#### Error Context
```rust
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ErrorContext {
    pub operation: String,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Error {
    pub fn with_context(self, operation: impl Into<String>) -> Self {
        match self {
            Error::Unknown(msg) => Error::Unknown(format!("{}: {}", operation.into(), msg)),
            other => other,
        }
    }

    pub fn context(&self) -> ErrorContext {
        ErrorContext {
            operation: "unknown".to_string(),
            details: Some(self.to_string()),
            timestamp: Utc::now(),
        }
    }
}
```

### Configuration Management

#### Configuration Loading
```rust
use std::path::Path;

pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<DeployConfig> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(Error::Config(format!("Configuration file not found: {}", path.display())));
    }

    let contents = tokio::fs::read_to_string(path).await?;
    let config: DeployConfig = serde_json::from_str(&contents)?;

    config.validate()?;
    Ok(config)
}

pub async fn save_config<P: AsRef<Path>>(path: P, config: &DeployConfig) -> Result<()> {
    config.validate()?;

    let contents = serde_json::to_string_pretty(config)?;
    tokio::fs::write(path, contents).await?;

    Ok(())
}
```

#### Environment Variable Overrides
```rust
pub fn apply_env_overrides(config: &mut DeployConfig) -> Result<()> {
    if let Ok(hub_url) = std::env::var("ATLAS_HUB_URL") {
        config.hub_url = hub_url;
    }

    if let Ok(pack_id) = std::env::var("ATLAS_PACK_ID") {
        config.pack_id = pack_id;
    }

    if let Ok(channel) = std::env::var("ATLAS_CHANNEL") {
        config.channel = channel;
    }

    if let Ok(token) = std::env::var("ATLAS_DEPLOY_TOKEN") {
        config.deploy_token = token;
    }

    if let Ok(max_mem) = std::env::var("ATLAS_MAX_MEMORY_MB") {
        config.server_config.max_memory_mb = max_mem.parse()?;
    }

    if let Ok(auto_start) = std::env::var("ATLAS_AUTO_START") {
        config.auto_start = auto_start.parse()?;
    }

    Ok(())
}
```

### Validation

#### Input Validation
```rust
pub fn validate_pack_id(pack_id: &str) -> Result<()> {
    if pack_id.is_empty() {
        return Err(Error::Validation("Pack ID cannot be empty".to_string()));
    }

    if pack_id.len() > 100 {
        return Err(Error::Validation("Pack ID too long (max 100 characters)".to_string()));
    }

    // Allow alphanumeric, hyphens, and underscores
    let valid_chars = pack_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_');
    if !valid_chars {
        return Err(Error::Validation("Pack ID contains invalid characters".to_string()));
    }

    Ok(())
}

pub fn validate_memory_config(min_mb: u64, max_mb: u64) -> Result<()> {
    if min_mb == 0 {
        return Err(Error::Validation("Minimum memory must be greater than 0".to_string()));
    }

    if max_mb < min_mb {
        return Err(Error::Validation("Maximum memory must be greater than minimum memory".to_string()));
    }

    if max_mb > 65536 { // 64GB limit
        return Err(Error::Validation("Maximum memory cannot exceed 64GB".to_string()));
    }

    Ok(())
}

pub fn validate_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(Error::Validation("URL must use HTTP or HTTPS".to_string()));
    }
    Ok(())
}
```

#### Server Properties Validation
```rust
impl ServerConfig {
    pub fn validate(&self) -> Result<()> {
        if let Some(max_players) = self.max_players {
            if max_players == 0 || max_players > 1000 {
                return Err(Error::Validation("Max players must be between 1 and 1000".to_string()));
            }
        }

        if let Some(view_distance) = self.view_distance {
            if view_distance < 3 || view_distance > 32 {
                return Err(Error::Validation("View distance must be between 3 and 32".to_string()));
            }
        }

        if let Some(simulation_distance) = self.simulation_distance {
            if simulation_distance < 3 || simulation_distance > 32 {
                return Err(Error::Validation("Simulation distance must be between 3 and 32".to_string()));
            }
        }

        if let Some(spawn_protection) = self.spawn_protection {
            if spawn_protection > 100 {
                return Err(Error::Validation("Spawn protection cannot exceed 100".to_string()));
            }
        }

        if let Some(rcon_port) = self.rcon_port {
            if rcon_port < 1024 || rcon_port > 65535 {
                return Err(Error::Validation("RCON port must be between 1024 and 65535".to_string()));
            }
        }

        Ok(())
    }
}
```

### Logging Utilities

#### Structured Logging
```rust
use std::fmt;

#[derive(Clone, Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub component: String,
    pub operation: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp: Utc::now(),
            component: "unknown".to_string(),
            operation: None,
            metadata: None,
        }
    }

    pub fn with_component(mut self, component: impl Into<String>) -> Self {
        self.component = component.into();
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} [{}] {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            self.level,
            self.component,
            self.message
        )
    }
}
```

#### Async Logging
```rust
use tokio::sync::mpsc;

pub struct AsyncLogger {
    sender: mpsc::UnboundedSender<LogEntry>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl AsyncLogger {
    pub fn new(log_path: impl AsRef<Path>) -> Result<Self> {
        let log_path = log_path.as_ref().to_path_buf();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let handle = tokio::spawn(async move {
            let file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .await;

            if let Ok(mut file) = file {
                while let Some(entry) = receiver.recv().await {
                    let line = format!("{}\n", entry);
                    let _ = file.write_all(line.as_bytes()).await;
                }
            }
        });

        Ok(Self {
            sender,
            handle: Some(handle),
        })
    }

    pub fn log(&self, entry: LogEntry) {
        let _ = self.sender.send(entry);
    }

    pub async fn shutdown(mut self) -> Result<()> {
        drop(self.sender);
        if let Some(handle) = self.handle.take() {
            handle.await?;
        }
        Ok(())
    }
}
```

### Utilities

#### UUID Generation
```rust
pub fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn generate_server_id() -> String {
    format!("server-{}", generate_id())
}

pub fn generate_backup_id() -> String {
    format!("backup-{}", generate_id())
}
```

#### Path Utilities
```rust
use std::path::{Path, PathBuf};

pub fn ensure_directory(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub async fn ensure_directory_async(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        tokio::fs::create_dir_all(path).await?;
    }
    Ok(())
}

pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}
```

#### Time Utilities
```rust
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

pub fn parse_duration(s: &str) -> Result<Duration> {
    let re = regex::Regex::new(r"^(\d+)([smhd])$")?;
    let captures = re.captures(s)
        .ok_or_else(|| Error::Validation(format!("Invalid duration format: {}", s)))?;

    let value: u64 = captures[1].parse()?;
    let unit = &captures[2];

    let seconds = match unit {
        "s" => value,
        "m" => value * 60,
        "h" => value * 3600,
        "d" => value * 86400,
        _ => return Err(Error::Validation(format!("Invalid time unit: {}", unit))),
    };

    Ok(Duration::from_secs(seconds))
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mod_loader_serialization() {
        let loader = ModLoader::Fabric;
        let json = serde_json::to_string(&loader).unwrap();
        assert_eq!(json, "\"fabric\"");

        let deserialized: ModLoader = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ModLoader::Fabric);
    }

    #[test]
    fn test_pack_id_validation() {
        assert!(validate_pack_id("valid-pack-123").is_ok());
        assert!(validate_pack_id("").is_err());
        assert!(validate_pack_id("invalid pack").is_err());
        assert!(validate_pack_id(&"a".repeat(101)).is_err());
    }

    #[test]
    fn test_memory_validation() {
        assert!(validate_memory_config(1024, 4096).is_ok());
        assert!(validate_memory_config(0, 1024).is_err());
        assert!(validate_memory_config(4096, 1024).is_err());
    }

    #[test]
    fn test_duration_formatting() {
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
        assert!(parse_duration("invalid").is_err());
    }
}
```

### Configuration Tests
```rust
#[cfg(test)]
mod config_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let config = DeployConfig {
            hub_url: "https://hub.example.com".to_string(),
            pack_id: "test-pack".to_string(),
            channel: "latest".to_string(),
            deploy_token: "secret-token".to_string(),
            server_config: ServerConfig::default(),
            auto_start: true,
            auto_update: true,
            backup_enabled: true,
            backup_interval_hours: 24,
            max_backups: 10,
            eula_accepted: true,
        };

        save_config(&config_path, &config).await.unwrap();
        let loaded = load_config(&config_path).await.unwrap();

        assert_eq!(loaded.hub_url, config.hub_url);
        assert_eq!(loaded.pack_id, config.pack_id);
    }

    #[test]
    fn test_env_overrides() {
        std::env::set_var("ATLAS_MAX_MEMORY_MB", "8192");

        let mut config = DeployConfig::default();
        apply_env_overrides(&mut config).unwrap();

        assert_eq!(config.server_config.max_memory_mb, 8192);

        std::env::remove_var("ATLAS_MAX_MEMORY_MB");
    }
}
```

### Error Handling Tests
```rust
#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let error = Error::Validation("test error".to_string());
        let context = error.context();

        assert_eq!(context.operation, "unknown");
        assert!(context.details.as_ref().unwrap().contains("test error"));
    }

    #[test]
    fn test_error_with_context() {
        let error = Error::Unknown("original".to_string());
        let contextualized = error.with_context("test operation");

        match contextualized {
            Error::Unknown(msg) => assert!(msg.contains("test operation")),
            _ => panic!("Expected Unknown error"),
        }
    }
}
```

## Usage Examples

### Configuration Management
```rust
use runner_core_v2::{DeployConfig, ServerConfig, ModLoader, Difficulty, GameMode};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a new deployment configuration
    let config = DeployConfig {
        hub_url: "https://hub.atlaslauncher.com".to_string(),
        pack_id: "my-awesome-pack".to_string(),
        channel: "stable".to_string(),
        deploy_token: "my-secret-token".to_string(),
        server_config: ServerConfig {
            max_memory_mb: 8192,
            min_memory_mb: 2048,
            difficulty: Some(Difficulty::Hard),
            gamemode: Some(GameMode::Survival),
            max_players: Some(50),
            motd: Some("Welcome to My Server!".to_string()),
            ..Default::default()
        },
        auto_start: true,
        auto_update: true,
        backup_enabled: true,
        backup_interval_hours: 6,
        max_backups: 20,
        eula_accepted: true,
    };

    // Validate configuration
    config.validate()?;

    // Save to file
    save_config("deploy.json", &config).await?;

    // Load and apply environment overrides
    let mut loaded = load_config("deploy.json").await?;
    apply_env_overrides(&mut loaded)?;

    println!("Configuration loaded successfully!");
    println!("Pack: {}", loaded.pack_id);
    println!("Memory: {}MB - {}MB", loaded.server_config.min_memory_mb, loaded.server_config.max_memory_mb);

    Ok(())
}
```

### Error Handling
```rust
use runner_core_v2::{Error, Result};

fn process_pack_data(data: &str) -> Result<Pack> {
    // Parse JSON
    let pack: Pack = serde_json::from_str(data)
        .map_err(|e| Error::Serialization(e).with_context("parsing pack JSON"))?;

    // Validate pack data
    if pack.id.is_empty() {
        return Err(Error::Validation("Pack ID cannot be empty".to_string())
            .with_context("validating pack data"));
    }

    // Validate loader
    match pack.loader {
        ModLoader::Vanilla | ModLoader::Fabric | ModLoader::Forge | ModLoader::NeoForge => {},
        _ => return Err(Error::Validation(format!("Unsupported mod loader: {}", pack.loader.as_str()))
            .with_context("validating mod loader")),
    }

    Ok(pack)
}

#[tokio::main]
async fn main() -> Result<()> {
    let json_data = r#"{"id": "", "name": "Test Pack", "loader": "invalid"}"#;

    match process_pack_data(json_data) {
        Ok(pack) => println!("Pack processed: {}", pack.name),
        Err(e) => {
            eprintln!("Error processing pack: {}", e);
            let context = e.context();
            eprintln!("Operation: {}", context.operation);
            if let Some(details) = context.details {
                eprintln!("Details: {}", details);
            }
        }
    }

    Ok(())
}
```

### Logging
```rust
use runner_core_v2::{AsyncLogger, LogEntry, LogLevel};

#[tokio::main]
async fn main() -> Result<()> {
    // Create async logger
    let logger = AsyncLogger::new("server.log").await?;

    // Log some events
    logger.log(LogEntry::new(LogLevel::Info, "Server starting")
        .with_component("server")
        .with_operation("startup"));

    logger.log(LogEntry::new(LogLevel::Warn, "High memory usage detected")
        .with_component("monitor")
        .with_metadata(serde_json::json!({
            "memory_mb": 7500,
            "threshold_mb": 8000
        })));

    logger.log(LogEntry::new(LogLevel::Error, "Failed to connect to database")
        .with_component("database")
        .with_operation("connect"));

    // Shutdown logger (ensures all logs are written)
    logger.shutdown().await?;

    Ok(())
}
```

## Maintenance

### Type Evolution
```rust
// Versioned types for backward compatibility
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "version")]
pub enum ServerConfigVersioned {
    #[serde(rename = "1")]
    V1(ServerConfigV1),
    #[serde(rename = "2")]
    V2(ServerConfig),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfigV1 {
    pub max_memory_mb: u64,
    pub min_memory_mb: u64,
    // Old fields...
}

impl From<ServerConfigV1> for ServerConfig {
    fn from(v1: ServerConfigV1) -> Self {
        ServerConfig {
            max_memory_mb: v1.max_memory_mb,
            min_memory_mb: v1.min_memory_mb,
            ..Default::default()
        }
    }
}
```

### Migration Utilities
```rust
pub async fn migrate_config(path: &Path) -> Result<()> {
    let contents = tokio::fs::read_to_string(path).await?;

    // Try to parse as versioned config
    if let Ok(versioned) = serde_json::from_str::<ServerConfigVersioned>(&contents) {
        let config = match versioned {
            ServerConfigVersioned::V1(v1) => ServerConfig::from(v1),
            ServerConfigVersioned::V2(v2) => v2,
        };

        // Save migrated config
        save_config(path, &config).await?;
        println!("Configuration migrated to latest version");
    }

    Ok(())
}
```

### Performance Monitoring
```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct Metrics {
    pub configs_loaded: AtomicU64,
    pub validations_performed: AtomicU64,
    pub errors_encountered: AtomicU64,
}

impl Metrics {
    pub fn record_config_load(&self) {
        self.configs_loaded.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_validation(&self) {
        self.validations_performed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors_encountered.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            configs_loaded: self.configs_loaded.load(Ordering::Relaxed),
            validations_performed: self.validations_performed.load(Ordering::Relaxed),
            errors_encountered: self.errors_encountered.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MetricsSnapshot {
    pub configs_loaded: u64,
    pub validations_performed: u64,
    pub errors_encountered: u64,
}
```