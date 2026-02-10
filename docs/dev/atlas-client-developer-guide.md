# Atlas Client Developer Documentation

The `atlas-client` crate provides HTTP client functionality for communicating with the Atlas Hub API, including authentication, pack management, and deployment operations.

## Architecture

### Dependencies
```toml
[dependencies]
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }  # HTTP client
serde = { version = "1.0", features = ["derive"] }                # Serialization
tokio = { version = "1.0", features = ["full"] }                  # Async runtime
anyhow = "1.0"                                                     # Error handling
url = "2.4"                                                        # URL parsing
base64 = "0.21"                                                    # Base64 encoding
oauth2 = { version = "4.4", features = ["reqwest"] }              # OAuth2 client
```

### Module Structure
```
src/
├── lib.rs              # Library exports
├── client.rs           # Main HTTP client
├── auth.rs             # Authentication handling
├── packs.rs            # Pack management API
├── deployments.rs      # Deployment operations
├── types.rs            # Shared type definitions
└── error.rs            # Error types
```

## Core Components

### HTTP Client

#### Client Configuration
```rust
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct HubClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl HubClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("atlas-client/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }
}
```

#### Request Building
```rust
impl HubClient {
    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.request(method, &url);

        // Add authorization header
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add JSON body if provided
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(HubError::Api {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let data: T = response.json().await?;
        Ok(data)
    }
}
```

### Authentication

#### OAuth2 Device Flow
```rust
impl HubClient {
    pub async fn start_device_auth(&self) -> Result<DeviceCodeResponse> {
        let response: DeviceCodeResponse = self.request(
            Method::POST,
            "/api/auth/device/code",
            None,
        ).await?;

        Ok(response)
    }

    pub async fn poll_device_token(
        &self,
        device_code: &DeviceCode,
    ) -> Result<AuthToken> {
        let body = json!({
            "device_code": device_code.device_code,
            "grant_type": "urn:ietf:params:oauth:grant-type:device_code"
        });

        let response: TokenResponse = self.request(
            Method::POST,
            "/api/auth/token",
            Some(body),
        ).await?;

        Ok(AuthToken {
            access_token: response.access_token,
            refresh_token: response.refresh_token,
            expires_in: response.expires_in,
        })
    }
}
```

#### Token Management
```rust
#[derive(Clone)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl AuthToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() > exp)
            .unwrap_or(false)
    }

    pub async fn refresh(&mut self, client: &HubClient) -> Result<()> {
        let refresh_token = self.refresh_token.as_ref()
            .ok_or(HubError::NoRefreshToken)?;

        let body = json!({
            "refresh_token": refresh_token,
            "grant_type": "refresh_token"
        });

        let response: TokenResponse = client.request(
            Method::POST,
            "/api/auth/token",
            Some(body),
        ).await?;

        self.access_token = response.access_token;
        self.refresh_token = response.refresh_token;
        self.expires_at = Some(Utc::now() + Duration::seconds(response.expires_in));

        Ok(())
    }
}
```

### Pack Management

#### Pack Operations
```rust
impl HubClient {
    pub async fn get_packs(&self) -> Result<Vec<PackSummary>> {
        self.request(Method::GET, "/api/packs", None).await
    }

    pub async fn get_pack(&self, pack_id: &str) -> Result<PackDetails> {
        let path = format!("/api/packs/{}", pack_id);
        self.request(Method::GET, &path, None).await
    }

    pub async fn create_pack(&self, pack: CreatePackRequest) -> Result<PackDetails> {
        self.request(Method::POST, "/api/packs", Some(json!(pack))).await
    }

    pub async fn update_pack(&self, pack_id: &str, pack: UpdatePackRequest) -> Result<PackDetails> {
        let path = format!("/api/packs/{}", pack_id);
        self.request(Method::PUT, &path, Some(json!(pack))).await
    }
}
```

#### Build Management
```rust
impl HubClient {
    pub async fn get_builds(&self, pack_id: &str) -> Result<Vec<BuildInfo>> {
        let path = format!("/api/packs/{}/builds", pack_id);
        self.request(Method::GET, &path, None).await
    }

    pub async fn get_build(&self, pack_id: &str, build_id: &str) -> Result<BuildDetails> {
        let path = format!("/api/packs/{}/builds/{}", pack_id, build_id);
        self.request(Method::GET, &path, None).await
    }

    pub async fn download_build(&self, pack_id: &str, build_id: &str) -> Result<Bytes> {
        let path = format!("/api/packs/{}/builds/{}/download", pack_id, build_id);

        let response = self.client
            .get(&format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bearer {}", self.token.as_ref().unwrap()))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(HubError::DownloadFailed(response.status().as_u16()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes)
    }
}
```

### Deployment Operations

#### Channel Management
```rust
impl HubClient {
    pub async fn get_channels(&self, pack_id: &str) -> Result<Vec<ChannelInfo>> {
        let path = format!("/api/packs/{}/channels", pack_id);
        self.request(Method::GET, &path, None).await
    }

    pub async fn promote_build(
        &self,
        pack_id: &str,
        channel: &str,
        build_id: &str,
    ) -> Result<()> {
        let path = format!("/api/packs/{}/channels/{}/promote", pack_id, channel);
        let body = json!({ "build_id": build_id });

        self.request::<Value>(Method::POST, &path, Some(body)).await?;
        Ok(())
    }
}
```

#### Deploy Token Management
```rust
impl HubClient {
    pub async fn create_deploy_token(
        &self,
        pack_id: &str,
        name: &str,
        permissions: Vec<String>,
    ) -> Result<DeployToken> {
        let path = format!("/api/packs/{}/tokens", pack_id);
        let body = json!({
            "name": name,
            "permissions": permissions
        });

        self.request(Method::POST, &path, Some(body)).await
    }

    pub async fn list_deploy_tokens(&self, pack_id: &str) -> Result<Vec<DeployToken>> {
        let path = format!("/api/packs/{}/tokens", pack_id);
        self.request(Method::GET, &path, None).await
    }

    pub async fn revoke_deploy_token(&self, pack_id: &str, token_id: &str) -> Result<()> {
        let path = format!("/api/packs/{}/tokens/{}", pack_id, token_id);
        self.request::<Value>(Method::DELETE, &path, None).await?;
        Ok(())
    }
}
```

## Error Handling

### Error Types
```rust
#[derive(thiserror::Error, Debug)]
pub enum HubError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Authentication required")]
    Unauthorized,

    #[error("Access denied")]
    Forbidden,

    #[error("Resource not found")]
    NotFound,

    #[error("Download failed with status: {0}")]
    DownloadFailed(u16),

    #[error("No refresh token available")]
    NoRefreshToken,

    #[error("OAuth2 error: {0}")]
    OAuth2(#[from] oauth2::Error<oauth2::reqwest::Error<reqwest::Error>>),
}
```

### Error Recovery
```rust
impl HubClient {
    pub async fn execute_with_retry<T, F, Fut>(
        &self,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let max_attempts = 3;

        loop {
            attempts += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err(HubError::Http(e)) if attempts < max_attempts => {
                    // Retry on network errors
                    if e.is_timeout() || e.is_connect() {
                        warn!("Request failed, retrying (attempt {}/{})", attempts, max_attempts);
                        tokio::time::sleep(Duration::from_millis(500 * attempts as u64)).await;
                        continue;
                    }
                }
                Err(HubError::Api { status: 401, .. }) => {
                    // Try to refresh token
                    if let Some(token) = &mut self.token {
                        if token.refresh().await.is_ok() {
                            continue; // Retry with new token
                        }
                    }
                    return Err(HubError::Unauthorized);
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

## Performance Considerations

### Connection Pooling
```rust
impl HubClient {
    pub fn with_connection_pool(mut self, max_connections: usize) -> Self {
        self.client = Client::builder()
            .pool_max_idle_per_host(max_connections)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("Failed to create HTTP client");

        self
    }
}
```

### Request Batching
```rust
impl HubClient {
    pub async fn batch_get_packs(&self, pack_ids: &[String]) -> Result<Vec<PackDetails>> {
        let futures: Vec<_> = pack_ids
            .iter()
            .map(|id| self.get_pack(id))
            .collect();

        let results = futures::future::join_all(futures).await;

        // Collect successful results, log errors
        let mut packs = Vec::new();
        for result in results {
            match result {
                Ok(pack) => packs.push(pack),
                Err(e) => warn!("Failed to fetch pack: {}", e),
            }
        }

        Ok(packs)
    }
}
```

### Response Caching
```rust
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CachedHubClient {
    client: HubClient,
    cache: Arc<RwLock<HashMap<String, (Value, DateTime<Utc>)>>>,
    ttl: Duration,
}

impl CachedHubClient {
    pub async fn get_pack_cached(&self, pack_id: &str) -> Result<PackDetails> {
        let cache_key = format!("pack:{}", pack_id);
        let now = Utc::now();

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((data, expires_at)) = cache.get(&cache_key) {
                if now < *expires_at {
                    return Ok(serde_json::from_value(data.clone())?);
                }
            }
        }

        // Fetch from API
        let pack = self.client.get_pack(pack_id).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            let expires_at = now + self.ttl;
            cache.insert(cache_key, (json!(pack), expires_at));
        }

        Ok(pack)
    }
}
```

## Security

### Certificate Validation
```rust
impl HubClient {
    pub fn with_custom_ca(mut self, ca_cert: &str) -> Result<Self> {
        let cert = reqwest::Certificate::from_pem(ca_cert.as_bytes())?;
        self.client = Client::builder()
            .add_root_certificate(cert)
            .build()?;

        Ok(self)
    }
}
```

### Request Signing
```rust
impl HubClient {
    pub fn with_request_signing(mut self, private_key: &str) -> Result<Self> {
        // Parse private key for request signing
        let key = parse_private_key(private_key)?;
        self.signing_key = Some(key);
        Ok(self)
    }

    fn sign_request(&self, method: &Method, url: &str, body: Option<&Value>) -> Result<String> {
        if let Some(key) = &self.signing_key {
            let timestamp = Utc::now().timestamp().to_string();
            let body_hash = if let Some(body) = body {
                sha256::digest(body.to_string().as_bytes())
            } else {
                String::new()
            };

            let message = format!("{}:{}:{}:{}", method, url, timestamp, body_hash);
            let signature = sign_message(key, message.as_bytes())?;

            Ok(format!("t={},s={}", timestamp, signature))
        } else {
            Ok(String::new())
        }
    }
}
```

## Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;

    #[tokio::test]
    async fn test_get_packs() {
        let mock_server = mock("GET", "/api/packs")
            .with_status(200)
            .with_body(r#"[{"id": "test-pack", "name": "Test Pack"}]"#)
            .create();

        let client = HubClient::new(mockito::server_url());
        let packs = client.get_packs().await.unwrap();

        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].id, "test-pack");

        mock_server.assert();
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
    async fn test_real_api() {
        if env::var("RUN_INTEGRATION_TESTS").is_err() {
            return; // Skip if not running integration tests
        }

        let base_url = env::var("HUB_URL").unwrap_or_else(|_| "https://hub.atlaslauncher.com".to_string());
        let client = HubClient::new(base_url);

        // Test basic connectivity
        let packs = client.get_packs().await;
        assert!(packs.is_ok() || matches!(packs, Err(HubError::Unauthorized)));
    }
}
```

### Mock Client for Testing
```rust
#[cfg(test)]
pub struct MockHubClient {
    packs: HashMap<String, PackDetails>,
    builds: HashMap<String, Vec<BuildInfo>>,
}

#[cfg(test)]
impl MockHubClient {
    pub fn new() -> Self {
        Self {
            packs: HashMap::new(),
            builds: HashMap::new(),
        }
    }

    pub fn add_pack(&mut self, pack: PackDetails) {
        self.packs.insert(pack.id.clone(), pack);
    }

    pub async fn get_pack(&self, pack_id: &str) -> Result<PackDetails> {
        self.packs.get(pack_id)
            .cloned()
            .ok_or(HubError::NotFound)
    }
}
```

## Usage Examples

### Basic Authentication Flow
```rust
use atlas_client::{HubClient, DeviceCode};

#[tokio::main]
async fn main() -> Result<()> {
    let client = HubClient::new("https://hub.atlaslauncher.com".to_string());

    // Start device authentication
    let device_code = client.start_device_auth().await?;

    println!("Visit: {}", device_code.verification_uri);
    println!("Code: {}", device_code.user_code);

    // Poll for completion
    let token = client.poll_device_token(&device_code).await?;

    // Create authenticated client
    let auth_client = client.with_token(token.access_token);

    // Use authenticated client
    let packs = auth_client.get_packs().await?;
    println!("Found {} packs", packs.len());

    Ok(())
}
```

### Pack Download and Installation
```rust
async fn download_and_install_pack(
    client: &HubClient,
    pack_id: &str,
    build_id: &str,
    install_path: &Path,
) -> Result<()> {
    // Download build
    let build_data = client.download_build(pack_id, build_id).await?;

    // Verify checksum if provided
    if let Some(expected_hash) = build.checksum {
        let actual_hash = sha256::digest(&build_data);
        if actual_hash != expected_hash {
            return Err(HubError::ChecksumMismatch);
        }
    }

    // Extract to installation directory
    extract_zip_archive(&build_data, install_path).await?;

    Ok(())
}
```

### Deployment Token Management
```rust
async fn setup_deployment_token(client: &HubClient, pack_id: &str) -> Result<String> {
    // Create deploy token with specific permissions
    let token = client.create_deploy_token(
        pack_id,
        "CI Deploy Token",
        vec!["build:read".to_string(), "channel:promote".to_string()],
    ).await?;

    println!("Created deploy token: {}", token.token);

    // Store token securely (not in code!)
    // save_deploy_token(&token.token)?;

    Ok(token.token)
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

### API Version Compatibility
```rust
impl HubClient {
    pub fn with_api_version(mut self, version: &str) -> Self {
        self.api_version = Some(version.to_string());
        self
    }

    fn add_api_version_header(&self, request: RequestBuilder) -> RequestBuilder {
        if let Some(version) = &self.api_version {
            request.header("X-API-Version", version)
        } else {
            request
        }
    }
}
```

### Monitoring and Metrics
```rust
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct ClientMetrics {
    pub requests_total: AtomicU64,
    pub requests_failed: AtomicU64,
    pub bytes_downloaded: AtomicU64,
    pub response_time_ms: AtomicU64,
}

impl HubClient {
    pub fn metrics(&self) -> &ClientMetrics {
        &self.metrics
    }

    async fn record_request(&self, start_time: Instant, success: bool, bytes: u64) {
        self.metrics.requests_total.fetch_add(1, Ordering::Relaxed);
        if !success {
            self.metrics.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
        self.metrics.bytes_downloaded.fetch_add(bytes, Ordering::Relaxed);

        let duration = start_time.elapsed().as_millis() as u64;
        self.metrics.response_time_ms.fetch_add(duration, Ordering::Relaxed);
    }
}
```