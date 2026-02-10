# Mod Resolver Developer Documentation

The `mod-resolver` crate handles downloading, caching, and verifying mod dependencies for Minecraft modpacks.

## Architecture

### Dependencies
```toml
[dependencies]
reqwest = { version = "0.11", features = ["stream"] }  # HTTP downloads
tokio = { version = "1.0", features = ["fs", "process"] } # Async I/O
futures = "0.3"                                        # Stream utilities
serde = { version = "1.0", features = ["derive"] }     # Serialization
anyhow = "1.0"                                          # Error handling
thiserror = "1.0"                                       # Error types
sha2 = "0.10"                                           # SHA-256 hashing
hex = "0.4"                                             # Hex encoding
url = "2.4"                                             # URL parsing
tempfile = "3.0"                                        # Temporary files
```

### Module Structure
```
src/
├── lib.rs              # Library exports
├── resolver.rs         # Main resolver implementation
├── cache.rs            # Download caching
├── downloader.rs       # HTTP download utilities
├── verifier.rs         # Hash verification
├── progress.rs         # Progress reporting
└── error.rs            # Error types
```

## Core Components

### Dependency Resolver

#### Resolver Configuration
```rust
#[derive(Clone)]
pub struct ResolverConfig {
    pub cache_dir: PathBuf,
    pub max_concurrent_downloads: usize,
    pub timeout: Duration,
    pub retry_attempts: u32,
    pub user_agent: String,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from("./cache"),
            max_concurrent_downloads: 4,
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
            user_agent: "Atlas-Mod-Resolver/1.0".to_string(),
        }
    }
}
```

#### Dependency Definition
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: Option<u64>,
    pub filename: Option<String>,
}

impl Dependency {
    pub fn new(name: String, url: String, sha256: String) -> Self {
        Self {
            name,
            url,
            sha256,
            size: None,
            filename: None,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_filename(mut self, filename: String) -> Self {
        self.filename = Some(filename);
        self
    }

    pub fn filename(&self) -> &str {
        self.filename.as_deref().unwrap_or_else(|| {
            // Extract filename from URL
            self.url.rsplit('/').next().unwrap_or("download")
        })
    }
}
```

#### Main Resolver
```rust
pub struct ModResolver {
    config: ResolverConfig,
    cache: Arc<Cache>,
    client: reqwest::Client,
}

impl ModResolver {
    pub fn new(config: ResolverConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent)
            .build()?;

        let cache = Arc::new(Cache::new(&config.cache_dir)?);

        Ok(Self {
            config,
            cache,
            client,
        })
    }

    pub async fn resolve_dependencies(
        &self,
        dependencies: &[Dependency],
        target_dir: &Path,
    ) -> Result<Vec<ResolvedDependency>> {
        // Create progress reporter
        let progress = Arc::new(ProgressReporter::new(dependencies.len()));

        // Resolve dependencies with concurrency limit
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_downloads));

        let tasks = dependencies.iter().enumerate().map(|(index, dep)| {
            let dep = dep.clone();
            let resolver = self.clone();
            let progress = progress.clone();
            let semaphore = semaphore.clone();
            let target_dir = target_dir.to_path_buf();

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                resolver.resolve_single_dependency(dep, &target_dir, progress, index).await
            })
        });

        // Collect results
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await??);
        }

        Ok(results)
    }

    async fn resolve_single_dependency(
        &self,
        dep: Dependency,
        target_dir: &Path,
        progress: Arc<ProgressReporter>,
        index: usize,
    ) -> Result<ResolvedDependency> {
        progress.start_dependency(index, &dep.name);

        // Check cache first
        if let Some(cached_path) = self.cache.get(&dep.sha256).await? {
            if self.verify_cached_file(&cached_path, &dep).await? {
                let target_path = target_dir.join(dep.filename());
                tokio::fs::copy(&cached_path, &target_path).await?;
                progress.complete_dependency(index);
                return Ok(ResolvedDependency {
                    dependency: dep,
                    local_path: target_path,
                    from_cache: true,
                });
            } else {
                // Cached file is corrupted, remove it
                tokio::fs::remove_file(&cached_path).await?;
            }
        }

        // Download the dependency
        let downloaded_path = self.download_dependency(&dep, progress.clone(), index).await?;

        // Cache the downloaded file
        self.cache.put(&dep.sha256, &downloaded_path).await?;

        // Copy to target directory
        let target_path = target_dir.join(dep.filename());
        tokio::fs::copy(&downloaded_path, &target_path).await?;

        progress.complete_dependency(index);

        Ok(ResolvedDependency {
            dependency: dep,
            local_path: target_path,
            from_cache: false,
        })
    }
}
```

### Download Cache

#### Cache Implementation
```rust
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct Cache {
    cache_dir: PathBuf,
    index: RwLock<HashMap<String, PathBuf>>, // sha256 -> path
}

impl Cache {
    pub async fn new(cache_dir: &Path) -> Result<Self> {
        tokio::fs::create_dir_all(cache_dir).await?;

        let mut index = HashMap::new();

        // Load existing cache index
        let index_path = cache_dir.join("index.json");
        if index_path.exists() {
            let index_data = tokio::fs::read(&index_path).await?;
            index = serde_json::from_slice(&index_data)?;
        }

        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            index: RwLock::new(index),
        })
    }

    pub async fn get(&self, sha256: &str) -> Result<Option<PathBuf>> {
        let index = self.index.read().await;
        Ok(index.get(sha256).cloned())
    }

    pub async fn put(&self, sha256: &str, source_path: &Path) -> Result<()> {
        let cache_filename = format!("{}.cached", sha256);
        let cache_path = self.cache_dir.join(&cache_filename);

        // Copy file to cache
        tokio::fs::copy(source_path, &cache_path).await?;

        // Update index
        {
            let mut index = self.index.write().await;
            index.insert(sha256.to_string(), cache_path.clone());
        }

        // Save index to disk
        self.save_index().await?;

        Ok(())
    }

    pub async fn cleanup(&self, max_size_bytes: u64) -> Result<u64> {
        let mut index = self.index.write().await;
        let mut total_size = 0u64;

        // Calculate current cache size
        for path in index.values() {
            if let Ok(metadata) = tokio::fs::metadata(path).await {
                total_size += metadata.len();
            }
        }

        if total_size <= max_size_bytes {
            return Ok(0);
        }

        // Remove oldest files until under limit
        let mut entries: Vec<_> = index.iter().collect();
        entries.sort_by_key(|(_, path)| {
            tokio::fs::metadata(path).map(|m| m.modified()).unwrap_or(Ok(SystemTime::UNIX_EPOCH)).unwrap_or(SystemTime::UNIX_EPOCH)
        });

        let mut removed_size = 0u64;
        for (sha256, path) in entries {
            if total_size - removed_size <= max_size_bytes {
                break;
            }

            if let Ok(metadata) = tokio::fs::metadata(path).await {
                let file_size = metadata.len();
                if tokio::fs::remove_file(path).await.is_ok() {
                    removed_size += file_size;
                    index.remove(sha256);
                }
            }
        }

        // Save updated index
        self.save_index().await?;

        Ok(removed_size)
    }

    async fn save_index(&self) -> Result<()> {
        let index_path = self.cache_dir.join("index.json");
        let index = self.index.read().await;
        let index_data = serde_json::to_vec_pretty(&*index)?;
        tokio::fs::write(&index_path, index_data).await?;
        Ok(())
    }
}
```

### HTTP Downloader

#### Download with Progress
```rust
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        target_path: &Path,
        mut progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(DownloadProgress) + Send + 'static,
    {
        let response = self.client.get(url).send().await?;
        let total_size = response.content_length();

        let mut file = tokio::fs::File::create(target_path).await?;
        let mut stream = response.bytes_stream();

        let mut downloaded = 0u64;
        let mut last_update = Instant::now();
        let mut last_downloaded = 0u64;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            // Update progress periodically
            let now = Instant::now();
            if now.duration_since(last_update) >= Duration::from_millis(100) {
                let speed_bps = (downloaded - last_downloaded) as f64 /
                    now.duration_since(last_update).as_secs_f64();

                progress_callback(DownloadProgress {
                    downloaded,
                    total: total_size,
                    speed_bps,
                });

                last_update = now;
                last_downloaded = downloaded;
            }
        }

        // Final progress update
        progress_callback(DownloadProgress {
            downloaded,
            total: total_size,
            speed_bps: 0.0,
        });

        file.flush().await?;
        Ok(())
    }
}
```

#### Retry Logic
```rust
pub async fn download_with_retry(
    &self,
    url: &str,
    target_path: &Path,
    max_attempts: u32,
    progress_callback: impl Fn(DownloadProgress) + Send + 'static,
) -> Result<()> {
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match self.download_with_progress(url, target_path, &progress_callback).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_attempts {
                    // Exponential backoff
                    let delay = Duration::from_millis(500 * 2u64.pow(attempt - 1));
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

### Verification System

#### Hash Verification
```rust
pub async fn verify_file_sha256(path: &Path, expected_sha256: &str) -> Result<bool> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = sha2::Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let actual_sha256 = hex::encode(hasher.finalize());
    Ok(actual_sha256 == expected_sha256)
}

pub async fn verify_file_size(path: &Path, expected_size: u64) -> Result<bool> {
    let metadata = tokio::fs::metadata(path).await?;
    Ok(metadata.len() == expected_size)
}
```

#### Comprehensive Verification
```rust
pub async fn verify_dependency(&self, path: &Path, dep: &Dependency) -> Result<()> {
    // Verify SHA-256 hash
    if !verify_file_sha256(path, &dep.sha256).await? {
        return Err(Error::HashMismatch {
            expected: dep.sha256.clone(),
            path: path.to_path_buf(),
        });
    }

    // Verify size if provided
    if let Some(expected_size) = dep.size {
        if !verify_file_size(path, expected_size).await? {
            return Err(Error::SizeMismatch {
                expected: expected_size,
                path: path.to_path_buf(),
            });
        }
    }

    Ok(())
}
```

### Progress Reporting

#### Progress Reporter
```rust
use tokio::sync::mpsc;

pub struct ProgressReporter {
    total_dependencies: usize,
    completed: AtomicUsize,
    sender: mpsc::UnboundedSender<ProgressEvent>,
}

#[derive(Clone, Debug)]
pub enum ProgressEvent {
    DependencyStarted { index: usize, name: String },
    DependencyProgress { index: usize, downloaded: u64, total: Option<u64> },
    DependencyCompleted { index: usize },
    AllCompleted,
}

impl ProgressReporter {
    pub fn new(total_dependencies: usize) -> (Self, mpsc::UnboundedReceiver<ProgressEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        (
            Self {
                total_dependencies,
                completed: AtomicUsize::new(0),
                sender,
            },
            receiver,
        )
    }

    pub fn start_dependency(&self, index: usize, name: &str) {
        let _ = self.sender.send(ProgressEvent::DependencyStarted {
            index,
            name: name.to_string(),
        });
    }

    pub fn update_progress(&self, index: usize, downloaded: u64, total: Option<u64>) {
        let _ = self.sender.send(ProgressEvent::DependencyProgress {
            index,
            downloaded,
            total,
        });
    }

    pub fn complete_dependency(&self, index: usize) {
        let completed = self.completed.fetch_add(1, Ordering::SeqCst) + 1;

        let _ = self.sender.send(ProgressEvent::DependencyCompleted { index });

        if completed == self.total_dependencies {
            let _ = self.sender.send(ProgressEvent::AllCompleted);
        }
    }
}
```

## Error Handling

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Download failed for {url}: {source}")]
    DownloadFailed { url: String, source: reqwest::Error },

    #[error("Hash verification failed for {path}: expected {expected}")]
    HashMismatch { expected: String, path: PathBuf },

    #[error("Size verification failed for {path}: expected {expected} bytes")]
    SizeMismatch { expected: u64, path: PathBuf },

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Dependency resolution failed: {0}")]
    ResolutionFailed(String),
}
```

### Error Recovery
```rust
impl ModResolver {
    pub async fn resolve_with_fallback(
        &self,
        dependencies: &[Dependency],
        target_dir: &Path,
    ) -> Result<Vec<ResolvedDependency>> {
        let mut results = Vec::new();
        let mut failed_deps = Vec::new();

        // Try to resolve all dependencies
        for dep in dependencies {
            match self.resolve_single_dependency(dep.clone(), target_dir, progress, 0).await {
                Ok(resolved) => results.push(resolved),
                Err(e) => {
                    warn!("Failed to resolve dependency {}: {}", dep.name, e);
                    failed_deps.push(dep.clone());
                }
            }
        }

        // If some failed, try alternative URLs if available
        if !failed_deps.is_empty() {
            for dep in failed_deps {
                if let Some(alt_url) = self.find_alternative_url(&dep).await? {
                    let mut alt_dep = dep;
                    alt_dep.url = alt_url;

                    match self.resolve_single_dependency(alt_dep, target_dir, progress, 0).await {
                        Ok(resolved) => results.push(resolved),
                        Err(e) => {
                            error!("Failed to resolve dependency {} with fallback: {}", dep.name, e);
                            return Err(e);
                        }
                    }
                } else {
                    return Err(Error::ResolutionFailed(format!("No fallback URL for {}", dep.name)));
                }
            }
        }

        Ok(results)
    }

    async fn find_alternative_url(&self, dep: &Dependency) -> Result<Option<String>> {
        // Check for mirror URLs, CDN fallbacks, etc.
        // This could query a mirror service or use hardcoded fallbacks
        Ok(None) // Placeholder
    }
}
```

## Performance Considerations

### Concurrent Downloads
```rust
use tokio::sync::Semaphore;

pub async fn resolve_parallel(
    &self,
    dependencies: &[Dependency],
    target_dir: &Path,
    max_concurrent: usize,
) -> Result<Vec<ResolvedDependency>> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let mut tasks = Vec::new();

    for dep in dependencies {
        let dep = dep.clone();
        let resolver = Arc::new(self.clone());
        let semaphore = semaphore.clone();
        let target_dir = target_dir.to_path_buf();

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            resolver.resolve_single_dependency(dep, &target_dir).await
        });

        tasks.push(task);
    }

    let results = futures::future::join_all(tasks).await;
    let mut resolved = Vec::new();

    for result in results {
        resolved.push(result??);
    }

    Ok(resolved)
}
```

### Connection Pooling
```rust
impl ModResolver {
    pub fn with_connection_pool(mut self, max_connections: usize) -> Result<Self> {
        self.client = reqwest::Client::builder()
            .pool_max_idle_per_host(max_connections)
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(60))
            .build()?;

        Ok(self)
    }
}
```

### Bandwidth Throttling
```rust
use tokio::sync::Semaphore;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct BandwidthLimiter {
    bytes_per_second: u64,
    current_window: AtomicU64,
    window_start: AtomicU64,
}

impl BandwidthLimiter {
    pub fn new(bytes_per_second: u64) -> Self {
        Self {
            bytes_per_second,
            current_window: AtomicU64::new(0),
            window_start: AtomicU64::new(unix_timestamp()),
        }
    }

    pub async fn wait_for_bandwidth(&self, bytes: u64) {
        loop {
            let now = unix_timestamp();
            let window_start = self.window_start.load(Ordering::SeqCst);

            if now - window_start >= 1 {
                // New window
                self.current_window.store(bytes, Ordering::SeqCst);
                self.window_start.store(now, Ordering::SeqCst);
                break;
            } else {
                let current = self.current_window.load(Ordering::SeqCst);
                if current + bytes <= self.bytes_per_second {
                    self.current_window.store(current + bytes, Ordering::SeqCst);
                    break;
                } else {
                    // Wait for next window
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
}
```

## Security

### URL Validation
```rust
pub fn validate_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;

    // Only allow HTTP/HTTPS
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(Error::InvalidUrl(format!("Unsupported scheme: {}", parsed.scheme())));
    }

    // Prevent localhost/private IP access
    if let Some(host) = parsed.host_str() {
        if host == "localhost" || host.starts_with("127.") ||
           host.starts_with("10.") || host.starts_with("172.") ||
           host.starts_with("192.168.") {
            return Err(Error::InvalidUrl("Private/localhost URLs not allowed".to_string()));
        }
    }

    Ok(())
}
```

### File Type Validation
```rust
pub fn validate_file_type(path: &Path, allowed_extensions: &[&str]) -> Result<()> {
    if let Some(extension) = path.extension() {
        let ext_str = extension.to_str().unwrap_or("");
        if !allowed_extensions.contains(&ext_str) {
            return Err(Error::InvalidFileType(ext_str.to_string()));
        }
    }

    Ok(())
}
```

### Sandboxed Downloads
```rust
pub async fn download_to_temp_dir(&self, url: &str) -> Result<PathBuf> {
    // Create temporary directory
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join("download.tmp");

    // Download to temp file
    self.downloader.download_with_progress(url, &temp_path, |_| {}).await?;

    // Verify the download
    // ... hash verification ...

    // Move to final location (atomic)
    let final_path = self.cache_dir.join("verified.tmp");
    tokio::fs::rename(&temp_path, &final_path).await?;

    // Clean up temp directory
    temp_dir.close()?;

    Ok(final_path)
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use mockito::mock;

    #[tokio::test]
    async fn test_dependency_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("cache");

        let config = ResolverConfig {
            cache_dir,
            max_concurrent_downloads: 2,
            ..Default::default()
        };

        let resolver = ModResolver::new(config).unwrap();

        // Mock dependency
        let dep = Dependency::new(
            "test-mod".to_string(),
            "http://example.com/mod.jar".to_string(),
            "abc123...".to_string(),
        );

        // This would need a real HTTP server or mock
        // let result = resolver.resolve_single_dependency(dep, temp_dir.path()).await;
        // assert!(result.is_ok());
    }

    #[test]
    fn test_url_validation() {
        assert!(validate_url("https://example.com/mod.jar").is_ok());
        assert!(validate_url("http://example.com/mod.jar").is_ok());
        assert!(validate_url("ftp://example.com/mod.jar").is_err());
        assert!(validate_url("http://localhost/mod.jar").is_err());
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
    async fn test_real_download() {
        if env::var("RUN_INTEGRATION_TESTS").is_err() {
            return;
        }

        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = ResolverConfig::default();
        let resolver = ModResolver::new(config).unwrap();

        // Test with a small, stable file
        let dep = Dependency::new(
            "test-file".to_string(),
            "https://httpbin.org/bytes/1024".to_string(),
            "expected_sha256_here".to_string(),
        );

        let target_dir = temp_dir.path().join("mods");
        tokio::fs::create_dir(&target_dir).await.unwrap();

        let result = resolver.resolve_single_dependency(dep, &target_dir).await;
        assert!(result.is_ok());
    }
}
```

### Mock Server Tests
```rust
#[cfg(test)]
mod mock_tests {
    use super::*;
    use mockito::{Server, Mock};

    #[tokio::test]
    async fn test_download_with_mock() {
        let mut server = Server::new_async().await;
        let url = server.url();

        // Mock a file download
        let _mock = server
            .mock("GET", "/test.jar")
            .with_body("fake jar content")
            .create();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = ResolverConfig::default();
        let resolver = ModResolver::new(config).unwrap();

        let dep = Dependency::new(
            "test-mod".to_string(),
            format!("{}/test.jar", url),
            sha256::digest(b"fake jar content").to_string(),
        );

        let target_dir = temp_dir.path().join("mods");
        tokio::fs::create_dir(&target_dir).await.unwrap();

        let result = resolver.resolve_single_dependency(dep, &target_dir).await;
        assert!(result.is_ok());
    }
}
```

## Usage Examples

### Basic Dependency Resolution
```rust
use mod_resolver::{ModResolver, ResolverConfig, Dependency};

#[tokio::main]
async fn main() -> Result<()> {
    // Create resolver
    let config = ResolverConfig {
        cache_dir: PathBuf::from("./cache"),
        max_concurrent_downloads: 4,
        ..Default::default()
    };

    let resolver = ModResolver::new(config)?;

    // Define dependencies
    let dependencies = vec![
        Dependency::new(
            "fabric-api".to_string(),
            "https://example.com/fabric-api.jar".to_string(),
            "abc123...".to_string(),
        ),
        Dependency::new(
            "my-mod".to_string(),
            "https://example.com/my-mod.jar".to_string(),
            "def456...".to_string(),
        ),
    ];

    // Resolve dependencies
    let target_dir = PathBuf::from("./mods");
    let resolved = resolver.resolve_dependencies(&dependencies, &target_dir).await?;

    println!("Resolved {} dependencies", resolved.len());
    for dep in resolved {
        println!("  {} -> {}", dep.dependency.name, dep.local_path.display());
    }

    Ok(())
}
```

### Progress Reporting
```rust
use mod_resolver::ProgressEvent;

#[tokio::main]
async fn main() -> Result<()> {
    let config = ResolverConfig::default();
    let resolver = ModResolver::new(config)?;

    let dependencies = vec![/* ... */];

    // Create progress reporter
    let (progress, mut receiver) = ProgressReporter::new(dependencies.len());

    // Spawn progress monitoring task
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                ProgressEvent::DependencyStarted { index, name } => {
                    println!("Starting download of {}", name);
                }
                ProgressEvent::DependencyProgress { index, downloaded, total } => {
                    if let Some(total) = total {
                        let percent = (downloaded as f64 / total as f64) * 100.0;
                        println!("Dependency {}: {:.1}%", index, percent);
                    }
                }
                ProgressEvent::DependencyCompleted { index } => {
                    println!("Dependency {} completed", index);
                }
                ProgressEvent::AllCompleted => {
                    println!("All downloads completed!");
                }
            }
        }
    });

    // Resolve with progress reporting
    let resolved = resolver.resolve_dependencies_with_progress(
        &dependencies,
        &target_dir,
        progress,
    ).await?;

    Ok(())
}
```

### Cache Management
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let config = ResolverConfig::default();
    let resolver = ModResolver::new(config)?;

    // Clean up cache if it gets too large
    let max_cache_size = 1024 * 1024 * 1024; // 1GB
    let cleaned_bytes = resolver.cache.cleanup(max_cache_size).await?;
    println!("Cleaned {} bytes from cache", cleaned_bytes);

    // Pre-warm cache with commonly used mods
    let common_deps = vec![/* frequently used dependencies */];
    for dep in common_deps {
        let _ = resolver.resolve_single_dependency(dep, &temp_dir).await;
    }

    Ok(())
}
```

## Maintenance

### Dependency Updates
```bash
# Check for updates
cargo outdated

# Update dependencies
cargo update

# Test after updates
cargo test
```

### Cache Maintenance
```rust
impl ModResolver {
    pub async fn maintenance(&self) -> Result<MaintenanceReport> {
        let mut report = MaintenanceReport::default();

        // Clean up orphaned cache entries
        report.orphaned_files = self.cache.cleanup_orphaned().await?;

        // Verify cache integrity
        report.corrupted_files = self.cache.verify_integrity().await?;

        // Update cache statistics
        report.total_size = self.cache.calculate_size().await?;
        report.file_count = self.cache.count_files().await?;

        Ok(report)
    }
}

#[derive(Default)]
pub struct MaintenanceReport {
    pub orphaned_files: usize,
    pub corrupted_files: usize,
    pub total_size: u64,
    pub file_count: usize,
}
```

### Monitoring and Metrics
```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct ResolverMetrics {
    pub downloads_total: AtomicU64,
    pub downloads_failed: AtomicU64,
    pub bytes_downloaded: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
}

impl ModResolver {
    pub fn record_download(&self, success: bool, bytes: u64, from_cache: bool) {
        self.metrics.downloads_total.fetch_add(1, Ordering::Relaxed);
        self.metrics.bytes_downloaded.fetch_add(bytes, Ordering::Relaxed);

        if !success {
            self.metrics.downloads_failed.fetch_add(1, Ordering::Relaxed);
        }

        if from_cache {
            self.metrics.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.metrics.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn get_metrics(&self) -> ResolverMetricsSnapshot {
        ResolverMetricsSnapshot {
            downloads_total: self.metrics.downloads_total.load(Ordering::Relaxed),
            downloads_failed: self.metrics.downloads_failed.load(Ordering::Relaxed),
            bytes_downloaded: self.metrics.bytes_downloaded.load(Ordering::Relaxed),
            cache_hits: self.metrics.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.metrics.cache_misses.load(Ordering::Relaxed),
        }
    }
}
```