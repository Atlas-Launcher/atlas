# Protocol Developer Documentation

The `protocol` crate handles Protocol Buffers serialization, Zstd compression, and provides utilities for working with Atlas's binary distribution format.

## Architecture

### Dependencies
```toml
[dependencies]
prost = "0.12"                                    # Protocol Buffers
prost-types = "0.12"                              # Protobuf types
zstd = "0.13"                                     # Zstd compression
serde = { version = "1.0", features = ["derive"] } # Serialization
anyhow = "1.0"                                     # Error handling
thiserror = "1.0"                                  # Error types
sha2 = "0.10"                                      # SHA-256 hashing
hex = "0.4"                                        # Hex encoding
tokio = { version = "1.0", features = ["fs"] }     # Async file I/O
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
├── pack.rs             # Pack serialization/deserialization
├── manifest.rs         # Manifest handling
├── compression.rs      # Zstd compression utilities
├── hashing.rs          # SHA-256 hashing utilities
├── validation.rs       # Data validation
└── error.rs            # Error types

proto/
├── pack.proto          # Pack message definitions
├── manifest.proto      # Manifest message definitions
└── common.proto        # Common types

build.rs                # Protobuf code generation
```

## Core Components

### Protocol Buffers Definitions

#### Pack Message Structure
```protobuf
// proto/pack.proto
syntax = "proto3";

package atlas.pack;

import "common.proto";

message Pack {
    string id = 1;
    string name = 2;
    string version = 3;
    string minecraft_version = 4;
    string loader = 5;  // "fabric", "forge", "neoforge"
    repeated string authors = 6;
    string description = 7;
    string website_url = 8;

    // Build metadata
    string build_id = 9;
    uint64 build_timestamp = 10;
    string commit_hash = 11;

    // Distribution
    Manifest manifest = 12;
    bytes payload = 13;  // Zstd-compressed virtual filesystem
}

message Manifest {
    repeated FileEntry files = 1;
    repeated Dependency dependencies = 2;
    repeated PlatformFilter platform_filters = 3;
}

message FileEntry {
    string path = 1;           // Relative path in virtual filesystem
    uint64 offset = 2;         // Offset in compressed payload
    uint64 size = 3;           // Uncompressed size
    string sha256 = 4;         // SHA-256 of uncompressed content
    FileMode mode = 5;         // File permissions
}

message Dependency {
    string name = 1;
    string url = 2;
    string sha256 = 3;
    uint64 size = 4;
    repeated PlatformFilter platform_filters = 5;
}

message PlatformFilter {
    Platform platform = 1;
    string architecture = 2;   // "x86_64", "aarch64"
    string os_family = 3;      // "unix", "windows"
}
```

#### Common Types
```protobuf
// proto/common.proto
syntax = "proto3";

package atlas.common;

enum Platform {
    PLATFORM_UNSPECIFIED = 0;
    PLATFORM_LINUX = 1;
    PLATFORM_MACOS = 2;
    PLATFORM_WINDOWS = 3;
}

enum FileMode {
    FILE_MODE_UNSPECIFIED = 0;
    FILE_MODE_REGULAR = 1;
    FILE_MODE_EXECUTABLE = 2;
    FILE_MODE_DIRECTORY = 3;
}
```

### Build Script
```rust
// build.rs
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = PathBuf::from("proto");

    // Generate Rust code from protobuf files
    prost_build::Config::new()
        .out_dir("src/")
        .compile_protos(
            &[
                proto_dir.join("pack.proto"),
                proto_dir.join("manifest.proto"),
                proto_dir.join("common.proto"),
            ],
            &[proto_dir],
        )?;

    Ok(())
}
```

### Pack Serialization

#### Pack Builder
```rust
use prost::Message;
use zstd::Encoder;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct PackBuilder {
    pack: pack::Pack,
    files: Vec<FileEntry>,
    compressor: Encoder<'static, Vec<u8>>,
}

impl PackBuilder {
    pub fn new(id: String, name: String, version: String) -> Result<Self> {
        let pack = pack::Pack {
            id,
            name,
            version,
            minecraft_version: String::new(),
            loader: String::new(),
            authors: Vec::new(),
            description: String::new(),
            website_url: String::new(),
            build_id: String::new(),
            build_timestamp: 0,
            commit_hash: String::new(),
            manifest: None,
            payload: Vec::new(),
        };

        let compressor = zstd::Encoder::new(Vec::new(), 19)?
            .auto_finish();

        Ok(Self {
            pack,
            files: Vec::new(),
            compressor,
        })
    }

    pub async fn add_file(&mut self, path: &str, content: &[u8]) -> Result<()> {
        // Calculate hash
        let hash = sha256::digest(content);

        // Compress content
        self.compressor.write_all(content).await?;
        let offset = self.files.last()
            .map(|f| f.offset + f.size)
            .unwrap_or(0);

        // Add file entry
        let entry = FileEntry {
            path: path.to_string(),
            offset,
            size: content.len() as u64,
            sha256: hash,
            mode: FileMode::Regular,
        };

        self.files.push(entry);
        Ok(())
    }

    pub async fn build(mut self) -> Result<pack::Pack> {
        // Finish compression
        self.compressor.flush().await?;
        let compressed_payload = self.compressor.finish()?;

        // Build manifest
        let manifest = pack::Manifest {
            files: self.files,
            dependencies: Vec::new(), // Add dependencies separately
            platform_filters: Vec::new(),
        };

        // Complete pack
        let mut pack = self.pack;
        pack.manifest = Some(manifest);
        pack.payload = compressed_payload;

        Ok(pack)
    }
}
```

#### Pack Reader
```rust
pub struct PackReader {
    pack: pack::Pack,
    decompressor: zstd::Decoder<'static, std::io::Cursor<&'static [u8]>>,
}

impl PackReader {
    pub fn new(pack: pack::Pack) -> Result<Self> {
        let decompressor = zstd::Decoder::new(std::io::Cursor::new(&pack.payload))?;

        Ok(Self { pack, decompressor })
    }

    pub async fn read_file(&mut self, path: &str) -> Result<Vec<u8>> {
        let entry = self.pack.manifest.as_ref()
            .and_then(|m| m.files.iter().find(|f| f.path == path))
            .ok_or_else(|| Error::FileNotFound(path.to_string()))?;

        // Seek to file offset
        self.decompressor.get_mut().set_position(entry.offset);

        // Read compressed data
        let mut compressed_data = vec![0u8; entry.size as usize];
        self.decompressor.read_exact(&mut compressed_data).await?;

        // Decompress
        let decompressed = zstd::decode_all(&compressed_data[..])?;

        // Verify hash
        let actual_hash = sha256::digest(&decompressed);
        if actual_hash != entry.sha256 {
            return Err(Error::HashMismatch {
                expected: entry.sha256.clone(),
                actual: actual_hash,
            });
        }

        Ok(decompressed)
    }

    pub fn list_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.pack.manifest.as_ref()
            .map(|m| m.files.iter())
            .into_iter()
            .flatten()
    }
}
```

### Compression Utilities

#### Zstd Configuration
```rust
pub struct CompressionConfig {
    pub level: i32,
    pub checksum: bool,
    pub dict: Option<Vec<u8>>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            level: 19,  // High compression
            checksum: true,
            dict: None,
        }
    }
}

pub async fn compress_data(data: &[u8], config: &CompressionConfig) -> Result<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), config.level)?;

    if config.checksum {
        encoder.include_checksum(true)?;
    }

    if let Some(dict) = &config.dict {
        encoder.set_dictionary(dict)?;
    }

    encoder.write_all(data).await?;
    let compressed = encoder.finish()?;

    Ok(compressed)
}

pub fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    zstd::decode_all(data)
}
```

#### Streaming Compression
```rust
pub struct StreamingCompressor<W: AsyncWrite + Unpin> {
    encoder: zstd::Encoder<W>,
    hasher: sha2::Sha256,
    bytes_written: u64,
}

impl<W: AsyncWrite + Unpin> StreamingCompressor<W> {
    pub fn new(writer: W, level: i32) -> Result<Self> {
        let encoder = zstd::Encoder::new(writer, level)?;
        let hasher = sha2::Sha256::new();

        Ok(Self {
            encoder,
            hasher,
            bytes_written: 0,
        })
    }

    pub async fn write_chunk(&mut self, data: &[u8]) -> Result<()> {
        self.encoder.write_all(data).await?;
        self.hasher.update(data);
        self.bytes_written += data.len() as u64;
        Ok(())
    }

    pub async fn finish(mut self) -> Result<(W, String)> {
        let writer = self.encoder.finish().await?;
        let hash = hex::encode(self.hasher.finalize());
        Ok((writer, hash))
    }
}
```

### Hashing and Validation

#### SHA-256 Utilities
```rust
use sha2::{Digest, Sha256};

pub fn sha256_digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub async fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_hash(data: &[u8], expected_hash: &str) -> bool {
    sha256_digest(data) == expected_hash
}
```

#### Manifest Validation
```rust
pub fn validate_manifest(manifest: &pack::Manifest) -> Result<()> {
    // Check for duplicate paths
    let mut seen_paths = std::collections::HashSet::new();
    for file in &manifest.files {
        if !seen_paths.insert(&file.path) {
            return Err(Error::DuplicatePath(file.path.clone()));
        }

        // Validate path format
        if file.path.contains("..") || file.path.starts_with('/') {
            return Err(Error::InvalidPath(file.path.clone()));
        }
    }

    // Validate dependencies
    for dep in &manifest.dependencies {
        if dep.url.is_empty() || dep.sha256.is_empty() {
            return Err(Error::InvalidDependency(dep.name.clone()));
        }
    }

    Ok(())
}
```

### Platform Filtering

#### Platform Detection
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub architecture: String,
    pub os_family: String,
}

pub fn detect_platform() -> PlatformInfo {
    let platform = if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else if cfg!(target_os = "windows") {
        Platform::Windows
    } else {
        Platform::Unspecified
    };

    let architecture = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    };

    let os_family = if cfg!(unix) {
        "unix"
    } else if cfg!(windows) {
        "windows"
    } else {
        "unknown"
    };

    PlatformInfo {
        platform,
        architecture: architecture.to_string(),
        os_family: os_family.to_string(),
    }
}
```

#### Filter Application
```rust
pub fn matches_platform_filter(
    filter: &pack::PlatformFilter,
    current_platform: &PlatformInfo,
) -> bool {
    // Check platform
    if filter.platform != Platform::Unspecified as i32 &&
       filter.platform != current_platform.platform as i32 {
        return false;
    }

    // Check architecture
    if !filter.architecture.is_empty() &&
       filter.architecture != current_platform.architecture {
        return false;
    }

    // Check OS family
    if !filter.os_family.is_empty() &&
       filter.os_family != current_platform.os_family {
        return false;
    }

    true
}

pub fn filter_dependencies(
    dependencies: &[pack::Dependency],
    platform: &PlatformInfo,
) -> Vec<&pack::Dependency> {
    dependencies
        .iter()
        .filter(|dep| {
            dep.platform_filters.is_empty() ||
            dep.platform_filters.iter().any(|f| matches_platform_filter(f, platform))
        })
        .collect()
}
```

## Error Handling

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    #[error("Zstd compression error: {0}")]
    Compression(#[from] std::io::Error),

    #[error("Zstd decompression error: {0}")]
    Decompression(#[from] std::io::Error),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Hash mismatch for file {path}: expected {expected}, got {actual}")]
    HashMismatch { path: String, expected: String, actual: String },

    #[error("Duplicate path in manifest: {0}")]
    DuplicatePath(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Invalid dependency: {0}")]
    InvalidDependency(String),

    #[error("Manifest validation failed: {0}")]
    Validation(String),
}
```

## Performance Considerations

### Memory-Mapped Files
```rust
use memmap2::Mmap;

pub struct MemoryMappedPack {
    pack: pack::Pack,
    mmap: Mmap,
}

impl MemoryMappedPack {
    pub async fn load(path: &Path) -> Result<Self> {
        // Read pack file
        let file = File::open(path).await?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Decode pack header
        let pack = pack::Pack::decode(&mmap)?;

        Ok(Self { pack, mmap })
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8]> {
        let entry = self.pack.manifest.as_ref()
            .and_then(|m| m.files.iter().find(|f| f.path == path))
            .ok_or_else(|| Error::FileNotFound(path.to_string()))?;

        let start = entry.offset as usize;
        let end = start + entry.size as usize;

        if end > self.mmap.len() {
            return Err(Error::InvalidOffset);
        }

        // Return slice from memory map
        Ok(&self.mmap[start..end])
    }
}
```

### Parallel Processing
```rust
use tokio::task;

pub async fn build_pack_parallel(
    files: Vec<(String, Vec<u8>)>,
    config: CompressionConfig,
) -> Result<pack::Pack> {
    // Compress files in parallel
    let compressed_files = futures::future::join_all(
        files.into_iter().map(|(path, content)| {
            task::spawn(async move {
                let compressed = compress_data(&content, &config).await?;
                Ok((path, content.len() as u64, compressed))
            })
        })
    ).await;

    // Build pack from compressed files
    let mut builder = PackBuilder::new(
        "pack-id".to_string(),
        "Pack Name".to_string(),
        "1.0.0".to_string(),
    )?;

    for result in compressed_files {
        let (path, original_size, compressed) = result??;
        builder.add_compressed_file(path, original_size, &compressed).await?;
    }

    builder.build().await
}
```

### Dictionary Compression
```rust
pub struct DictionaryCompressor {
    dict: zstd::dict::EncoderDictionary<'static>,
}

impl DictionaryCompressor {
    pub fn new(sample_data: &[u8]) -> Result<Self> {
        let dict = zstd::dict::from_continuous(
            &[sample_data],
            &zstd::dict::EncoderDictionaryConfig::default(),
        )?;

        Ok(Self { dict })
    }

    pub async fn compress_with_dict(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder = zstd::Encoder::new(Vec::new(), 19)?;
        encoder.set_dictionary(&self.dict)?;
        encoder.write_all(data).await?;
        let compressed = encoder.finish().await?;
        Ok(compressed)
    }
}
```

## Security

### Input Validation
```rust
pub fn validate_pack_data(data: &[u8]) -> Result<()> {
    // Check size limits
    const MAX_PACK_SIZE: u64 = 100 * 1024 * 1024; // 100MB
    if data.len() as u64 > MAX_PACK_SIZE {
        return Err(Error::PackTooLarge);
    }

    // Decode and validate structure
    let pack = pack::Pack::decode(data)?;

    // Validate manifest
    if let Some(manifest) = &pack.manifest {
        validate_manifest(manifest)?;
    }

    // Validate payload size
    if pack.payload.len() > MAX_PACK_SIZE as usize {
        return Err(Error::PayloadTooLarge);
    }

    Ok(())
}
```

### Safe Path Handling
```rust
pub fn sanitize_path(path: &str) -> Result<String> {
    use std::path::Path;

    // Normalize path separators
    let normalized = path.replace('\\', "/");

    // Remove leading slashes
    let normalized = normalized.trim_start_matches('/');

    // Resolve . and ..
    let path_obj = Path::new(&normalized);
    let canonical = path_obj.canonicalize()?;

    // Ensure it's still relative
    if canonical.is_absolute() {
        return Err(Error::AbsolutePath);
    }

    // Convert back to string
    canonical.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::InvalidPath(path.to_string()))
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
    async fn test_pack_roundtrip() {
        let mut builder = PackBuilder::new(
            "test-pack".to_string(),
            "Test Pack".to_string(),
            "1.0.0".to_string(),
        ).unwrap();

        let test_content = b"Hello, World!";
        builder.add_file("hello.txt", test_content).await.unwrap();

        let pack = builder.build().await.unwrap();
        let mut reader = PackReader::new(pack).unwrap();

        let read_content = reader.read_file("hello.txt").await.unwrap();
        assert_eq!(read_content, test_content);
    }

    #[test]
    fn test_hash_verification() {
        let data = b"test data";
        let hash = sha256_digest(data);
        assert!(verify_hash(data, &hash));
        assert!(!verify_hash(b"different data", &hash));
    }
}
```

### Benchmark Tests
```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn bench_compression(c: &mut Criterion) {
        let data = vec![0u8; 1024 * 1024]; // 1MB of data

        c.bench_function("compress_1mb", |b| {
            b.iter(|| {
                black_box(compress_data(&data, &CompressionConfig::default()));
            })
        });
    }

    criterion_group!(benches, bench_compression);
    criterion_main!(benches);
}
```

### Fuzz Testing
```rust
#[cfg(test)]
mod fuzz_tests {
    use super::*;

    #[test]
    fn fuzz_pack_decode() {
        afl::fuzz!(|data: &[u8]| {
            let _ = pack::Pack::decode(data);
        });
    }

    #[test]
    fn fuzz_zstd_decompress() {
        afl::fuzz!(|data: &[u8]| {
            let _ = decompress_data(data);
        });
    }
}
```

## Usage Examples

### Creating a Pack
```rust
use protocol::{PackBuilder, CompressionConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Create pack builder
    let mut builder = PackBuilder::new(
        "my-modpack".to_string(),
        "My Modpack".to_string(),
        "1.0.0".to_string(),
    )?;

    // Set metadata
    builder.set_minecraft_version("1.20.1");
    builder.set_loader("fabric");
    builder.add_author("Author Name");

    // Add files
    builder.add_file("config/mod-config.toml", include_bytes!("../config.toml")).await?;
    builder.add_file("mods/my-mod.jar", include_bytes!("../my-mod.jar")).await?;

    // Add dependencies
    builder.add_dependency(Dependency {
        name: "fabric-api".to_string(),
        url: "https://example.com/fabric-api.jar".to_string(),
        sha256: "abc123...".to_string(),
        size: 1024,
        platform_filters: vec![],
    });

    // Build pack
    let pack = builder.build().await?;

    // Serialize to bytes
    let mut buf = Vec::new();
    pack.encode(&mut buf)?;

    // Write to file
    tokio::fs::write("my-pack.bin", buf).await?;

    Ok(())
}
```

### Reading a Pack
```rust
use protocol::PackReader;

#[tokio::main]
async fn main() -> Result<()> {
    // Read pack file
    let data = tokio::fs::read("my-pack.bin").await?;
    let pack = pack::Pack::decode(&data)?;

    // Create reader
    let mut reader = PackReader::new(pack)?;

    // List files
    println!("Files in pack:");
    for file in reader.list_files() {
        println!("  {} ({} bytes)", file.path, file.size);
    }

    // Read specific file
    let config = reader.read_file("config/mod-config.toml").await?;
    println!("Config content: {}", String::from_utf8_lossy(&config));

    Ok(())
}
```

### Applying Platform Filters
```rust
use protocol::{detect_platform, filter_dependencies};

#[tokio::main]
async fn main() -> Result<()> {
    let pack_data = tokio::fs::read("pack.bin").await?;
    let pack = pack::Pack::decode(&pack_data)?;

    let current_platform = detect_platform();

    if let Some(manifest) = &pack.manifest {
        let applicable_deps = filter_dependencies(&manifest.dependencies, &current_platform);

        println!("Applicable dependencies for current platform:");
        for dep in applicable_deps {
            println!("  {}: {}", dep.name, dep.url);
        }
    }

    Ok(())
}
```

## Maintenance

### Protocol Evolution
```rust
// Version compatibility checks
impl pack::Pack {
    pub fn protocol_version(&self) -> u32 {
        // Extract version from build metadata or use default
        1
    }

    pub fn is_compatible(&self) -> bool {
        let version = self.protocol_version();
        version >= MIN_SUPPORTED_VERSION && version <= MAX_SUPPORTED_VERSION
    }
}

const MIN_SUPPORTED_VERSION: u32 = 1;
const MAX_SUPPORTED_VERSION: u32 = 1;
```

### Performance Monitoring
```rust
use std::time::Instant;

pub struct PerformanceMetrics {
    pub compression_time: Duration,
    pub decompression_time: Duration,
    pub bytes_processed: u64,
}

pub async fn compress_with_metrics(
    data: &[u8],
    config: &CompressionConfig,
) -> Result<(Vec<u8>, PerformanceMetrics)> {
    let start = Instant::now();
    let compressed = compress_data(data, config).await?;
    let compression_time = start.elapsed();

    let metrics = PerformanceMetrics {
        compression_time,
        decompression_time: Duration::default(), // Not measured here
        bytes_processed: data.len() as u64,
    };

    Ok((compressed, metrics))
}
```

### Debugging Utilities
```rust
pub fn dump_pack_info(pack: &pack::Pack) {
    println!("Pack ID: {}", pack.id);
    println!("Name: {}", pack.name);
    println!("Version: {}", pack.version);
    println!("Minecraft Version: {}", pack.minecraft_version);
    println!("Loader: {}", pack.loader);

    if let Some(manifest) = &pack.manifest {
        println!("Files: {}", manifest.files.len());
        println!("Dependencies: {}", manifest.dependencies.len());

        println!("File listing:");
        for file in &manifest.files {
            println!("  {}: {} bytes, hash: {}", file.path, file.size, &file.sha256[..8]);
        }
    }

    println!("Payload size: {} bytes", pack.payload.len());
}
```