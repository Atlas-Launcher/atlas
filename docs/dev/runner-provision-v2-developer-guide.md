# Runner Provision v2 Developer Documentation

The `runner-provision-v2` crate handles Minecraft server provisioning, including pack application, dependency resolution, and JVM launch configuration.

## Architecture

### Dependencies
```toml
[dependencies]
tokio = { version = "1.0", features = ["fs", "process"] } # Async I/O and processes
serde = { version = "1.0", features = ["derive"] }       # Serialization
anyhow = "1.0"                                           # Error handling
thiserror = "1.0"                                        # Error types
tempfile = "3.0"                                         # Temporary files
regex = "1.0"                                            # Regular expressions
walkdir = "2.0"                                          # Directory traversal
mod-resolver = { path = "../mod-resolver" }             # Dependency resolution
protocol = { path = "../protocol" }                     # Pack handling
runner-core-v2 = { path = "../runner-core-v2" }         # Core types
```

### Module Structure
```
src/
├── lib.rs              # Library exports
├── provisioner.rs      # Main provisioner implementation
├── jvm.rs              # JVM launch configuration
├── pack.rs             # Pack application logic
├── server.rs           # Server process management
├── validation.rs       # Configuration validation
└── error.rs            # Error types
```

## Core Components

### Server Provisioner

#### Provisioner Configuration
```rust
#[derive(Clone, Debug)]
pub struct ProvisionerConfig {
    pub server_dir: PathBuf,
    pub java_home: Option<PathBuf>,
    pub min_memory_mb: u64,
    pub max_memory_mb: u64,
    pub java_args: Vec<String>,
    pub server_args: Vec<String>,
    pub auto_restart: bool,
    pub restart_delay_seconds: u32,
}

impl Default for ProvisionerConfig {
    fn default() -> Self {
        Self {
            server_dir: PathBuf::from("./server"),
            java_home: None,
            min_memory_mb: 1024,
            max_memory_mb: 4096,
            java_args: vec![
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:+AlwaysPreTouch".to_string(),
            ],
            server_args: vec!["nogui".to_string()],
            auto_restart: true,
            restart_delay_seconds: 10,
        }
    }
}
```

#### Main Provisioner
```rust
pub struct ServerProvisioner {
    config: ProvisionerConfig,
    pack_resolver: Arc<PackResolver>,
    mod_resolver: Arc<ModResolver>,
}

impl ServerProvisioner {
    pub fn new(config: ProvisionerConfig) -> Result<Self> {
        let pack_resolver = Arc::new(PackResolver::new()?);
        let mod_resolver = Arc::new(ModResolver::new(Default::default())?);

        Ok(Self {
            config,
            pack_resolver,
            mod_resolver,
        })
    }

    pub async fn provision_server(&self, pack_data: &[u8]) -> Result<ProvisionedServer> {
        // Create server directory
        ensure_directory(&self.config.server_dir).await?;

        // Load and validate pack
        let pack = self.pack_resolver.load_pack(pack_data).await?;

        // Apply pack files
        self.apply_pack(&pack).await?;

        // Resolve and install dependencies
        self.resolve_dependencies(&pack).await?;

        // Generate server properties
        self.generate_server_properties(&pack).await?;

        // Create launch script
        let launch_script = self.create_launch_script(&pack).await?;

        Ok(ProvisionedServer {
            server_dir: self.config.server_dir.clone(),
            pack_info: pack,
            launch_script,
        })
    }

    pub async fn start_server(&self, provisioned: &ProvisionedServer) -> Result<RunningServer> {
        // Validate server can start
        self.validate_server_ready(provisioned).await?;

        // Launch JVM process
        let jvm_config = self.build_jvm_config(provisioned).await?;
        let process = self.launch_jvm(&jvm_config).await?;

        // Monitor startup
        let monitor = ServerMonitor::new(process, self.config.clone());
        let running = monitor.start().await?;

        Ok(running)
    }
}
```

### Pack Application

#### Pack Resolver
```rust
pub struct PackResolver {
    temp_dir: TempDir,
}

impl PackResolver {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    pub async fn load_pack(&self, pack_data: &[u8]) -> Result<PackInfo> {
        // Decode pack
        let pack = protocol::Pack::decode(pack_data)?;

        // Validate pack structure
        self.validate_pack(&pack).await?;

        // Extract pack info
        let pack_info = PackInfo {
            id: pack.id,
            name: pack.name,
            version: pack.version,
            minecraft_version: pack.minecraft_version,
            loader: pack.loader,
            manifest: pack.manifest.unwrap_or_default(),
        };

        Ok(pack_info)
    }

    async fn validate_pack(&self, pack: &protocol::Pack) -> Result<()> {
        // Check required fields
        if pack.id.is_empty() || pack.name.is_empty() || pack.version.is_empty() {
            return Err(ProvisionError::InvalidPack("Missing required pack metadata".to_string()));
        }

        // Validate manifest
        if let Some(manifest) = &pack.manifest {
            protocol::validate_manifest(manifest)?;
        }

        // Check payload size
        if pack.payload.len() > MAX_PACK_SIZE {
            return Err(ProvisionError::PackTooLarge);
        }

        Ok(())
    }
}
```

#### Pack Application Logic
```rust
impl ServerProvisioner {
    async fn apply_pack(&self, pack: &PackInfo) -> Result<()> {
        let server_dir = &self.config.server_dir;

        // Create backup of existing files
        self.backup_existing_files(server_dir).await?;

        // Apply pack files from manifest
        for file in &pack.manifest.files {
            self.apply_pack_file(pack, file, server_dir).await?;
        }

        // Apply configuration templates
        self.apply_config_templates(pack, server_dir).await?;

        Ok(())
    }

    async fn apply_pack_file(
        &self,
        pack: &PackInfo,
        file: &protocol::FileEntry,
        server_dir: &Path,
    ) -> Result<()> {
        let target_path = server_dir.join(&file.path);

        // Ensure parent directory exists
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Read file from pack payload
        let content = pack.read_file(&file.path).await?;

        // Verify hash
        let actual_hash = sha256::digest(&content);
        if actual_hash != file.sha256 {
            return Err(ProvisionError::HashMismatch {
                path: file.path.clone(),
                expected: file.sha256.clone(),
                actual: actual_hash,
            });
        }

        // Write file
        tokio::fs::write(&target_path, &content).await?;

        // Set permissions
        self.set_file_permissions(&target_path, file.mode).await?;

        Ok(())
    }

    async fn set_file_permissions(&self, path: &Path, mode: i32) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = match mode {
                1 => 0o755, // executable
                _ => 0o644, // regular file
            };
            let mut perms = tokio::fs::metadata(path).await?.permissions();
            perms.set_mode(permissions);
            tokio::fs::set_permissions(path, perms).await?;
        }
        Ok(())
    }
}
```

### Dependency Resolution

#### Dependency Installation
```rust
impl ServerProvisioner {
    async fn resolve_dependencies(&self, pack: &PackInfo) -> Result<()> {
        let mods_dir = self.config.server_dir.join("mods");
        tokio::fs::create_dir_all(&mods_dir).await?;

        // Collect all dependencies
        let dependencies = self.collect_dependencies(pack).await?;

        // Resolve dependencies
        let resolved = self.mod_resolver.resolve_dependencies(
            &dependencies,
            &mods_dir,
        ).await?;

        // Log results
        info!("Resolved {} dependencies", resolved.len());
        for dep in resolved {
            if dep.from_cache {
                info!("  {} (cached)", dep.dependency.name);
            } else {
                info!("  {} (downloaded)", dep.dependency.name);
            }
        }

        Ok(())
    }

    async fn collect_dependencies(&self, pack: &PackInfo) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Add dependencies from manifest
        for dep in &pack.manifest.dependencies {
            // Check platform compatibility
            if self.is_platform_compatible(dep).await? {
                dependencies.push(Dependency {
                    name: dep.name.clone(),
                    url: dep.url.clone(),
                    sha256: dep.sha256.clone(),
                    size: Some(dep.size),
                    filename: None,
                });
            }
        }

        // Add loader-specific dependencies
        match pack.loader {
            ModLoader::Fabric => {
                self.add_fabric_dependencies(&mut dependencies, pack).await?;
            }
            ModLoader::Forge => {
                self.add_forge_dependencies(&mut dependencies, pack).await?;
            }
            ModLoader::NeoForge => {
                self.add_neoforge_dependencies(&mut dependencies, pack).await?;
            }
            _ => {}
        }

        Ok(dependencies)
    }

    async fn is_platform_compatible(&self, dep: &protocol::Dependency) -> Result<bool> {
        let current_platform = platform::detect_platform();

        // Check platform filters
        for filter in &dep.platform_filters {
            if protocol::matches_platform_filter(filter, &current_platform) {
                return Ok(true);
            }
        }

        // No filters means compatible with all platforms
        Ok(dep.platform_filters.is_empty())
    }
}
```

### JVM Launch Configuration

#### JVM Config Builder
```rust
#[derive(Clone, Debug)]
pub struct JvmConfig {
    pub java_executable: PathBuf,
    pub jvm_args: Vec<String>,
    pub jar_file: PathBuf,
    pub server_args: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: HashMap<String, String>,
}

impl ServerProvisioner {
    async fn build_jvm_config(&self, provisioned: &ProvisionedServer) -> Result<JvmConfig> {
        let java_exe = self.find_java_executable().await?;
        let jar_file = self.find_server_jar(&provisioned.server_dir).await?;

        let mut jvm_args = self.config.java_args.clone();

        // Memory settings
        jvm_args.push(format!("-Xmx{}M", self.config.max_memory_mb));
        jvm_args.push(format!("-Xms{}M", self.config.min_memory_mb));

        // Add loader-specific JVM arguments
        self.add_loader_jvm_args(&provisioned.pack_info, &mut jvm_args).await?;

        // Environment variables
        let mut environment = HashMap::new();
        environment.insert("JAVA_HOME".to_string(),
            java_exe.parent().unwrap().parent().unwrap().to_string_lossy().to_string());

        Ok(JvmConfig {
            java_executable: java_exe,
            jvm_args,
            jar_file,
            server_args: self.config.server_args.clone(),
            working_directory: provisioned.server_dir.clone(),
            environment,
        })
    }

    async fn find_java_executable(&self) -> Result<PathBuf> {
        // Check configured JAVA_HOME
        if let Some(java_home) = &self.config.java_home {
            let java_exe = java_home.join("bin").join("java");
            if java_exe.exists() {
                return Ok(java_exe);
            }
        }

        // Check PATH
        if let Ok(java_exe) = which::which("java") {
            return Ok(java_exe);
        }

        // Check common locations
        for location in &["/usr/bin/java", "/usr/local/bin/java", "/opt/java/bin/java"] {
            let path = PathBuf::from(location);
            if path.exists() {
                return Ok(path);
            }
        }

        Err(ProvisionError::JavaNotFound)
    }

    async fn find_server_jar(&self, server_dir: &Path) -> Result<PathBuf> {
        // Look for common server jar names
        let candidates = [
            "server.jar",
            "minecraft_server.jar",
            "fabric-server-launch.jar",
            "forge-*-universal.jar",
        ];

        for candidate in &candidates {
            let pattern = server_dir.join(candidate);
            if let Ok(mut entries) = tokio::fs::read_dir(server_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() &&
                       path.extension() == Some(OsStr::new("jar")) &&
                       self.matches_pattern(&path, candidate).await? {
                        return Ok(path);
                    }
                }
            }
        }

        Err(ProvisionError::ServerJarNotFound)
    }

    async fn matches_pattern(&self, path: &Path, pattern: &str) -> Result<bool> {
        let filename = path.file_name().unwrap().to_string_lossy();
        let regex_pattern = pattern.replace("*", ".*");
        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))?;
        Ok(regex.is_match(&filename))
    }
}
```

#### JVM Process Launch
```rust
impl ServerProvisioner {
    async fn launch_jvm(&self, config: &JvmConfig) -> Result<Child> {
        // Build command arguments
        let mut args = config.jvm_args.clone();
        args.push("-jar".to_string());
        args.push(config.jar_file.to_string_lossy().to_string());
        args.extend(config.server_args.clone());

        // Launch process
        let child = Command::new(&config.java_executable)
            .args(&args)
            .current_dir(&config.working_directory)
            .env_clear()
            .envs(&config.environment)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }
}
```

### Server Monitoring

#### Server Monitor
```rust
pub struct ServerMonitor {
    process: Child,
    config: ProvisionerConfig,
    state: Arc<RwLock<ServerState>>,
}

#[derive(Clone, Debug)]
pub enum ServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

impl ServerMonitor {
    pub fn new(process: Child, config: ProvisionerConfig) -> Self {
        Self {
            process,
            config,
            state: Arc::new(RwLock::new(ServerState::Starting)),
        }
    }

    pub async fn start(mut self) -> Result<RunningServer> {
        let stdout = self.process.stdout.take().unwrap();
        let stderr = self.process.stderr.take().unwrap();

        // Spawn log monitoring tasks
        let state = self.state.clone();
        tokio::spawn(async move {
            Self::monitor_logs(stdout, state.clone()).await;
        });

        tokio::spawn(async move {
            Self::monitor_logs(stderr, state).await;
        });

        // Wait for server to be ready
        self.wait_for_startup().await?;

        // Spawn restart monitor if enabled
        if self.config.auto_restart {
            let state = self.state.clone();
            let config = self.config.clone();
            tokio::spawn(async move {
                Self::monitor_restarts(state, config).await;
            });
        }

        Ok(RunningServer {
            process: self.process,
            state: self.state,
        })
    }

    async fn wait_for_startup(&self) -> Result<()> {
        let mut startup_timeout = Duration::from_secs(300); // 5 minutes
        let start_time = Instant::now();

        while start_time.elapsed() < startup_timeout {
            // Check if process is still running
            if let Ok(Some(_)) = self.process.try_wait() {
                return Err(ProvisionError::ServerCrashedOnStartup);
            }

            // Check for ready message in logs
            {
                let state = self.state.read().await;
                if matches!(*state, ServerState::Running) {
                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_millis(1000)).await;
        }

        Err(ProvisionError::StartupTimeout)
    }

    async fn monitor_logs(reader: impl AsyncRead + Unpin, state: Arc<RwLock<ServerState>>) {
        let mut lines = BufReader::new(reader).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            // Process log line
            if line.contains("Done (") && line.contains(")! For help, type \"help\"") {
                // Server is ready
                let mut state = state.write().await;
                *state = ServerState::Running;
            } else if line.contains("Stopping server") {
                let mut state = state.write().await;
                *state = ServerState::Stopping;
            }

            // Log the line
            info!("{}", line);
        }
    }

    async fn monitor_restarts(state: Arc<RwLock<ServerState>>, config: ProvisionerConfig) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let current_state = {
                let state = state.read().await;
                (*state).clone()
            };

            match current_state {
                ServerState::Running => {
                    // Server is healthy, continue monitoring
                }
                ServerState::Error(_) => {
                    warn!("Server in error state, attempting restart");
                    tokio::time::sleep(Duration::from_secs(config.restart_delay_seconds as u64)).await;

                    // Attempt restart logic would go here
                    // This is simplified for the example
                }
                _ => {}
            }
        }
    }
}
```

## Error Handling

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum ProvisionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pack error: {0}")]
    Pack(String),

    #[error("Invalid pack: {0}")]
    InvalidPack(String),

    #[error("Pack too large")]
    PackTooLarge,

    #[error("Hash mismatch for {path}: expected {expected}, got {actual}")]
    HashMismatch { path: String, expected: String, actual: String },

    #[error("Dependency resolution error: {0}")]
    Dependency(#[from] mod_resolver::Error),

    #[error("Java not found")]
    JavaNotFound,

    #[error("Server JAR not found")]
    ServerJarNotFound,

    #[error("Server crashed on startup")]
    ServerCrashedOnStartup,

    #[error("Server startup timeout")]
    StartupTimeout,

    #[error("Process error: {0}")]
    Process(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

## Performance Considerations

### Parallel File Operations
```rust
impl ServerProvisioner {
    async fn apply_pack_parallel(&self, pack: &PackInfo) -> Result<()> {
        let server_dir = &self.config.server_dir;
        let files = pack.manifest.files.clone();

        // Process files in parallel with limited concurrency
        let semaphore = Arc::new(Semaphore::new(10)); // Max 10 concurrent file operations
        let mut tasks = Vec::new();

        for file in files {
            let file = file.clone();
            let pack = pack.clone();
            let server_dir = server_dir.clone();
            let semaphore = semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                self.apply_pack_file(&pack, &file, &server_dir).await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            task.await??;
        }

        Ok(())
    }
}
```

### Memory-Mapped Pack Reading
```rust
use memmap2::Mmap;

pub struct MemoryMappedPack {
    pack: protocol::Pack,
    mmap: Mmap,
}

impl MemoryMappedPack {
    pub async fn load(path: &Path) -> Result<Self> {
        let file = tokio::fs::File::open(path).await?;
        let mmap = unsafe { Mmap::map(&file)? };
        let pack = protocol::Pack::decode(&mmap)?;

        Ok(Self { pack, mmap })
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8]> {
        let entry = self.pack.manifest.as_ref()
            .and_then(|m| m.files.iter().find(|f| f.path == path))
            .ok_or_else(|| ProvisionError::FileNotFound(path.to_string()))?;

        let start = entry.offset as usize;
        let end = start + entry.size as usize;

        if end > self.mmap.len() {
            return Err(ProvisionError::InvalidOffset);
        }

        Ok(&self.mmap[start..end])
    }
}
```

### Lazy Dependency Resolution
```rust
pub struct LazyDependencyResolver {
    resolver: ModResolver,
    resolved: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl LazyDependencyResolver {
    pub async fn resolve_on_demand(&self, dependency: &Dependency) -> Result<PathBuf> {
        let key = format!("{}:{}", dependency.name, dependency.sha256);

        // Check if already resolved
        {
            let resolved = self.resolved.read().await;
            if let Some(path) = resolved.get(&key) {
                return Ok(path.clone());
            }
        }

        // Resolve dependency
        let temp_dir = tempfile::TempDir::new()?;
        let resolved_deps = self.resolver.resolve_dependencies(
            &[dependency.clone()],
            temp_dir.path(),
        ).await?;

        let path = resolved_deps[0].local_path.clone();

        // Cache result
        {
            let mut resolved = self.resolved.write().await;
            resolved.insert(key, path.clone());
        }

        Ok(path)
    }
}
```

## Security

### File Validation
```rust
impl ServerProvisioner {
    async fn validate_pack_files(&self, pack: &PackInfo) -> Result<()> {
        for file in &pack.manifest.files {
            // Prevent directory traversal
            if file.path.contains("..") || file.path.starts_with('/') {
                return Err(ProvisionError::InvalidPack(format!("Invalid file path: {}", file.path)));
            }

            // Check for dangerous file types
            if self.is_dangerous_file(&file.path).await? {
                return Err(ProvisionError::InvalidPack(format!("Dangerous file type: {}", file.path)));
            }

            // Validate file size
            if file.size > MAX_FILE_SIZE {
                return Err(ProvisionError::InvalidPack(format!("File too large: {}", file.path)));
            }
        }

        Ok(())
    }

    async fn is_dangerous_file(&self, path: &str) -> Result<bool> {
        let dangerous_patterns = [
            ".sh", ".bat", ".cmd", ".exe", ".dll", ".so", ".dylib",
            "server.properties", "ops.json", "whitelist.json",
        ];

        let path_lower = path.to_lowercase();
        Ok(dangerous_patterns.iter().any(|pattern| path_lower.ends_with(pattern)))
    }
}
```

### Sandboxed Execution
```rust
impl ServerProvisioner {
    async fn create_sandbox(&self, server_dir: &Path) -> Result<Sandbox> {
        // Create isolated directory structure
        let sandbox_dir = server_dir.join("sandbox");
        tokio::fs::create_dir_all(&sandbox_dir).await?;

        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&sandbox_dir).await?.permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&sandbox_dir, perms).await?;
        }

        Ok(Sandbox {
            root_dir: sandbox_dir,
            allowed_paths: vec![server_dir.to_path_buf()],
        })
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
    async fn test_pack_validation() {
        let resolver = PackResolver::new().unwrap();

        // Valid pack
        let valid_pack = protocol::Pack {
            id: "test-pack".to_string(),
            name: "Test Pack".to_string(),
            version: "1.0.0".to_string(),
            minecraft_version: "1.20.1".to_string(),
            loader: "fabric".to_string(),
            authors: vec![],
            description: None,
            website_url: None,
            build_id: String::new(),
            build_timestamp: 0,
            commit_hash: None,
            manifest: Some(protocol::Manifest::default()),
            payload: vec![],
        };

        assert!(resolver.validate_pack(&valid_pack).await.is_ok());

        // Invalid pack (missing required fields)
        let invalid_pack = protocol::Pack {
            id: String::new(), // Empty ID
            ..valid_pack
        };

        assert!(resolver.validate_pack(&invalid_pack).await.is_err());
    }

    #[test]
    fn test_platform_compatibility() {
        let provisioner = ServerProvisioner::new(Default::default()).unwrap();

        let compatible_dep = protocol::Dependency {
            name: "test-mod".to_string(),
            url: "http://example.com/mod.jar".to_string(),
            sha256: "abc123".to_string(),
            size: 1024,
            platform_filters: vec![],
        };

        // Should be compatible (no filters)
        assert!(provisioner.is_platform_compatible(&compatible_dep).unwrap());
    }
}
```

### Integration Tests
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_full_provisioning() {
        if env::var("RUN_INTEGRATION_TESTS").is_err() {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let config = ProvisionerConfig {
            server_dir: temp_dir.path().join("server"),
            ..Default::default()
        };

        let provisioner = ServerProvisioner::new(config).unwrap();

        // Load a test pack (would need actual pack data)
        // let pack_data = include_bytes!("test_pack.bin");
        // let provisioned = provisioner.provision_server(pack_data).await.unwrap();

        // Verify server directory structure
        // assert!(provisioned.server_dir.exists());
        // assert!(provisioned.server_dir.join("mods").exists());
    }
}
```

### Mock Components
```rust
#[cfg(test)]
pub struct MockModResolver {
    dependencies: HashMap<String, PathBuf>,
}

#[cfg(test)]
impl MockModResolver {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    pub fn add_dependency(&mut self, name: &str, path: PathBuf) {
        self.dependencies.insert(name.to_string(), path);
    }
}

#[cfg(test)]
#[async_trait::async_trait]
impl ModResolver for MockModResolver {
    async fn resolve_dependencies(
        &self,
        dependencies: &[Dependency],
        target_dir: &Path,
    ) -> Result<Vec<ResolvedDependency>> {
        let mut results = Vec::new();

        for dep in dependencies {
            if let Some(path) = self.dependencies.get(&dep.name) {
                let target_path = target_dir.join(&dep.name);
                tokio::fs::copy(path, &target_path).await?;

                results.push(ResolvedDependency {
                    dependency: dep.clone(),
                    local_path: target_path,
                    from_cache: true,
                });
            }
        }

        Ok(results)
    }
}
```

## Usage Examples

### Basic Server Provisioning
```rust
use runner_provision_v2::{ServerProvisioner, ProvisionerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Create provisioner
    let config = ProvisionerConfig {
        server_dir: PathBuf::from("./my-server"),
        max_memory_mb: 8192,
        min_memory_mb: 2048,
        auto_restart: true,
        ..Default::default()
    };

    let provisioner = ServerProvisioner::new(config)?;

    // Load pack data (from hub download)
    let pack_data = download_pack_from_hub().await?;
    let provisioned = provisioner.provision_server(&pack_data).await?;

    println!("Server provisioned in: {}", provisioned.server_dir.display());
    println!("Pack: {} v{}", provisioned.pack_info.name, provisioned.pack_info.version);

    // Start the server
    let running = provisioner.start_server(&provisioned).await?;
    println!("Server started with PID: {}", running.process.id().unwrap());

    Ok(())
}
```

### Custom JVM Configuration
```rust
use runner_provision_v2::ProvisionerConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ProvisionerConfig {
        server_dir: PathBuf::from("./server"),
        java_home: Some(PathBuf::from("/opt/java17")),
        max_memory_mb: 16384,
        min_memory_mb: 4096,
        java_args: vec![
            "-XX:+UseG1GC".to_string(),
            "-XX:MaxGCPauseMillis=100".to_string(),
            "-XX:+UnlockExperimentalVMOptions".to_string(),
            "-XX:+DisableExplicitGC".to_string(),
            "-XX:G1NewSizePercent=20".to_string(),
            "-XX:G1MaxNewSizePercent=30".to_string(),
            "-Dlog4j2.formatMsgNoLookups=true".to_string(), // Security
        ],
        server_args: vec![
            "nogui".to_string(),
            "--port".to_string(),
            "25565".to_string(),
        ],
        auto_restart: true,
        restart_delay_seconds: 15,
    };

    let provisioner = ServerProvisioner::new(config)?;
    // ... provision and start server

    Ok(())
}
```

### Monitoring Server Health
```rust
use runner_provision_v2::RunningServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Assume we have a running server
    let running_server: RunningServer = /* ... */;

    // Monitor server health
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let state = {
            let state = running_server.state.read().await;
            (*state).clone()
        };

        match state {
            ServerState::Running => {
                println!("Server is healthy");
            }
            ServerState::Error(msg) => {
                eprintln!("Server error: {}", msg);
                // Could trigger restart logic here
            }
            ServerState::Stopped => {
                println!("Server stopped");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Maintenance

### Configuration Migration
```rust
pub async fn migrate_legacy_config(path: &Path) -> Result<()> {
    let contents = tokio::fs::read_to_string(path).await?;

    // Try to parse as legacy format
    if let Ok(legacy) = serde_json::from_str::<LegacyConfig>(&contents) {
        let config = ProvisionerConfig::from(legacy);
        save_config(path, &config).await?;
        println!("Configuration migrated from legacy format");
    }

    Ok(())
}

#[derive(Deserialize)]
struct LegacyConfig {
    pub server_dir: PathBuf,
    pub memory_mb: u64, // Old single memory field
    // ... other legacy fields
}

impl From<LegacyConfig> for ProvisionerConfig {
    fn from(legacy: LegacyConfig) -> Self {
        Self {
            server_dir: legacy.server_dir,
            max_memory_mb: legacy.memory_mb,
            min_memory_mb: legacy.memory_mb / 2,
            ..Default::default()
        }
    }
}
```

### Performance Metrics
```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct ProvisionMetrics {
    pub packs_provisioned: AtomicU64,
    pub dependencies_resolved: AtomicU64,
    pub servers_started: AtomicU64,
    pub startup_time_ms: AtomicU64,
    pub errors_encountered: AtomicU64,
}

impl ProvisionMetrics {
    pub fn record_provision(&self, duration: Duration, success: bool) {
        self.packs_provisioned.fetch_add(1, Ordering::Relaxed);
        self.startup_time_ms.fetch_add(duration.as_millis() as u64, Ordering::Relaxed);

        if !success {
            self.errors_encountered.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_dependency_resolution(&self, count: u64) {
        self.dependencies_resolved.fetch_add(count, Ordering::Relaxed);
    }

    pub fn record_server_start(&self) {
        self.servers_started.fetch_add(1, Ordering::Relaxed);
    }
}
```

### Health Checks
```rust
impl ServerProvisioner {
    pub async fn health_check(&self, provisioned: &ProvisionedServer) -> Result<HealthStatus> {
        let mut status = HealthStatus::Healthy;

        // Check server directory exists
        if !provisioned.server_dir.exists() {
            return Ok(HealthStatus::Unhealthy("Server directory missing".to_string()));
        }

        // Check server jar exists
        let jar_path = self.find_server_jar(&provisioned.server_dir).await;
        if jar_path.is_err() {
            return Ok(HealthStatus::Unhealthy("Server JAR missing".to_string()));
        }

        // Check mods directory
        let mods_dir = provisioned.server_dir.join("mods");
        if !mods_dir.exists() {
            return Ok(HealthStatus::Unhealthy("Mods directory missing".to_string()));
        }

        // Check for required configuration files
        let config_files = ["server.properties", "eula.txt"];
        for file in &config_files {
            let file_path = provisioned.server_dir.join(file);
            if !file_path.exists() {
                return Ok(HealthStatus::Unhealthy(format!("Required file missing: {}", file)));
            }
        }

        Ok(status)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
}
```