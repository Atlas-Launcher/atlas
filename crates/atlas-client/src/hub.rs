use anyhow::{Context, Result};
use reqwest::{header, Client, Response};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use url::Url;

use crate::device_code::{
    hub_device_code_endpoint, hub_device_token_endpoint, parse_device_token_poll_body,
    DeviceCodeRequest, DeviceCodeResponse, DeviceTokenPollStatus, DeviceTokenRequest,
    StandardDeviceTokenResponse, DEFAULT_ATLAS_DEVICE_CLIENT_ID, DEFAULT_ATLAS_DEVICE_SCOPE,
};

pub struct HubClient {
    client: Client,
    base_url: Url,
    auth: Mutex<AuthState>,
    pack_deploy_token: Mutex<Option<String>>,
}

#[derive(Clone, Debug)]
struct AccessToken {
    value: String,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
struct ServiceAuthState {
    service_token: String,
    access_token: Option<AccessToken>,
}

#[derive(Clone, Debug)]
enum AuthState {
    None,
    UserToken(String),
    ServiceToken(ServiceAuthState),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackMetadataResponse {
    pub pack_id: String,
    pub channel: String,
    pub build_id: String,
    pub version: Option<String>,
    pub minecraft_version: Option<String>,
    pub modloader: Option<String>,
    pub modloader_version: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherPack {
    pub pack_id: String,
    pub pack_name: String,
    pub pack_slug: String,
    pub repo_url: Option<String>,
    pub channel: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherArtifactResponse {
    #[serde(default)]
    pub pack_id: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    pub download_url: String,
    #[serde(default)]
    pub force_reinstall: Option<bool>,
    #[serde(default)]
    pub requires_full_reinstall: Option<bool>,
    #[serde(default)]
    pub minecraft_version: Option<String>,
    #[serde(default)]
    pub modloader: Option<String>,
    #[serde(default)]
    pub modloader_version: Option<String>,
    #[serde(default)]
    pub build_id: Option<String>,
    #[serde(default)]
    pub build_version: Option<String>,
}

#[derive(Debug)]
pub struct BuildBlobResult {
    pub bytes: Vec<u8>,
    pub force_reinstall: bool,
    pub requires_full_reinstall: bool,
    pub minecraft_version: Option<String>,
    pub modloader: Option<String>,
    pub modloader_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RunnerTokenExchange {
    pub access_token: String,
    pub expires_in: u64,
    pub pack_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RunnerServiceTokenRequest {
    pack_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerServiceTokenResponse {
    #[serde(default)]
    pub id: Option<String>,
    pub pack_id: String,
    pub token: String,
    pub prefix: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLinkSession {
    pub link_session_id: String,
    pub link_code: String,
    pub proof: String,
    pub expires_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLinkComplete {
    pub success: bool,
    pub user_id: String,
    #[serde(default)]
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MojangInfoResponse {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub uuid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LauncherPacksResponse {
    packs: Vec<LauncherPack>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistributionReleasePlatform {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistributionReleaseAsset {
    pub kind: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
    pub download_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistributionReleaseResponse {
    pub product: String,
    pub version: String,
    pub channel: String,
    pub published_at: String,
    pub platform: DistributionReleasePlatform,
    pub assets: Vec<DistributionReleaseAsset>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackBuild {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub commit_hash: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackChannel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub build_id: Option<String>,
    #[serde(default)]
    pub build_version: Option<String>,
    #[serde(default)]
    pub build_commit: Option<String>,
}

impl HubClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(30)).build()?,
            base_url,
            auth: Mutex::new(AuthState::None),
            pack_deploy_token: Mutex::new(None),
        })
    }

    pub fn set_token(&mut self, token: String) {
        let mut auth = self.auth.lock().expect("auth lock poisoned");
        *auth = AuthState::UserToken(token);
    }

    pub fn set_service_token(&mut self, token: String) {
        let mut auth = self.auth.lock().expect("auth lock poisoned");
        *auth = AuthState::ServiceToken(ServiceAuthState {
            service_token: token,
            access_token: None,
        });
    }

    pub fn set_pack_deploy_token(&mut self, token: String) {
        let mut value = self
            .pack_deploy_token
            .lock()
            .expect("pack_deploy_token lock poisoned");
        *value = Some(token);
    }

    fn get_pack_deploy_token(&self) -> Option<String> {
        self.pack_deploy_token
            .lock()
            .expect("pack_deploy_token lock poisoned")
            .clone()
    }

    async fn get_auth_headers(&self) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        let auth = { self.auth.lock().expect("auth lock poisoned").clone() };
        match auth {
            AuthState::None => return Ok(headers),
            AuthState::UserToken(token) => {
                headers.insert(
                    header::AUTHORIZATION,
                    header::HeaderValue::from_str(&format!("Bearer {}", token))?,
                );
            }
            AuthState::ServiceToken(state) => {
                let access = self.ensure_service_access_token(state).await?;
                headers.insert(
                    header::AUTHORIZATION,
                    header::HeaderValue::from_str(&format!("Bearer {}", access))?,
                );
            }
        }
        Ok(headers)
    }

    async fn ensure_service_access_token(&self, mut state: ServiceAuthState) -> Result<String> {
        if let Some(access) = &state.access_token {
            if Instant::now() + Duration::from_secs(60) < access.expires_at {
                return Ok(access.value.clone());
            }
        }

        let exchange = self.exchange_service_token(&state.service_token).await?;
        let access = AccessToken {
            value: exchange.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(exchange.expires_in.max(30)),
        };
        state.access_token = Some(access);

        let mut auth = self.auth.lock().expect("auth lock poisoned");
        *auth = AuthState::ServiceToken(state);

        Ok(exchange.access_token)
    }

    async fn exchange_service_token(&self, token: &str) -> Result<RunnerTokenExchange> {
        let url = self.base_url.join("/api/v1/runner/exchange")?;
        let response = self
            .client
            .post(url)
            .header("x-atlas-service-token", token)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let prefix_hint = if token.trim_start().starts_with("atlas_runner_") {
                ""
            } else {
                " (token does not start with expected runner prefix `atlas_runner_`)"
            };
            anyhow::bail!(
                "Runner service token exchange failed (HTTP {}): {}{}. \
Ensure you are using a runner service token from `/api/v1/runner/tokens`, not a pack/app deploy token.",
                status.as_u16(),
                body,
                prefix_hint
            );
        }

        response
            .json()
            .await
            .context("Failed to parse runner token exchange")
    }

    pub async fn validate_service_token(&self) -> Result<RunnerTokenExchange> {
        let auth = { self.auth.lock().expect("auth lock poisoned").clone() };
        match auth {
            AuthState::ServiceToken(state) => {
                self.exchange_service_token(&state.service_token).await
            }
            _ => anyhow::bail!("No service token configured"),
        }
    }

    pub async fn create_runner_service_token(
        &self,
        pack_id: &str,
        name: Option<String>,
    ) -> Result<RunnerServiceTokenResponse> {
        let url = self.base_url.join("/api/v1/runner/tokens")?;
        let response = self
            .client
            .post(url)
            .headers(self.get_auth_headers().await?)
            .json(&RunnerServiceTokenRequest {
                pack_id: pack_id.to_string(),
                name,
            })
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Runner token request failed (HTTP {}): {}",
                status.as_u16(),
                body
            );
        }

        let body = response
            .text()
            .await
            .context("Failed to read runner service token response")?;

        if std::env::var("ATLAS_DEBUG_RUNNER_TOKENS").is_ok() {
            eprintln!("Runner service token response: {body}");
        }

        let value = serde_json::from_str::<serde_json::Value>(&body).map_err(|err| {
            anyhow::anyhow!(
                "Failed to parse runner service token JSON: {}. Body: {}",
                err,
                body
            )
        })?;

        let pack_id = value
            .get("packId")
            .or_else(|| value.get("pack_id"))
            .and_then(|val| val.as_str())
            .map(|val| val.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Runner service token response missing packId. Body: {}",
                    body
                )
            })?;
        let token = value
            .get("token")
            .and_then(|val| val.as_str())
            .map(|val| val.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Runner service token response missing token. Body: {}",
                    body
                )
            })?;
        let prefix = value
            .get("prefix")
            .and_then(|val| val.as_str())
            .map(|val| val.to_string())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Runner service token response missing prefix. Body: {}",
                    body
                )
            })?;
        let id = value
            .get("id")
            .and_then(|val| val.as_str())
            .map(|val| val.to_string());

        Ok(RunnerServiceTokenResponse {
            id,
            pack_id,
            token,
            prefix,
        })
    }

    pub async fn get_pack_metadata(&self, pack_id: &str) -> Result<PackMetadata> {
        let url = self.base_url.join(&format!("/api/v1/packs/{pack_id}"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response
            .json()
            .await
            .context("Failed to parse pack metadata")
    }

    pub async fn list_launcher_packs(&self) -> Result<Vec<LauncherPack>> {
        let url = self.base_url.join("/api/v1/launcher/packs")?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response
            .json::<LauncherPacksResponse>()
            .await
            .context("Failed to parse launcher packs")
            .map(|payload| payload.packs)
    }

    pub async fn check_creator_permission(&self, pack_id: &str) -> Result<bool> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{pack_id}/access"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Deserialize)]
        struct AccessResponse {
            allowed: bool,
        }

        let access: AccessResponse = response.json().await?;
        Ok(access.allowed)
    }

    pub async fn get_launcher_artifact(
        &self,
        pack_id: &str,
        channel: &str,
        current_build_id: Option<&str>,
    ) -> Result<LauncherArtifactResponse> {
        let mut url = self
            .base_url
            .join(&format!("/api/v1/launcher/packs/{pack_id}/artifact"))?;
        url.query_pairs_mut().append_pair("channel", channel);
        if let Some(value) = current_build_id {
            url.query_pairs_mut().append_pair("currentBuildId", value);
        }

        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response
            .json()
            .await
            .context("Failed to parse launcher artifact response")
    }

    pub async fn download_blob(&self, download_url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(download_url)
            .send()
            .await?
            .error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_latest_distribution_release(
        &self,
        product: &str,
        os: &str,
        arch: &str,
    ) -> Result<DistributionReleaseResponse> {
        let url = self
            .base_url
            .join(&format!("/api/v1/releases/{product}/latest/{os}/{arch}"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response
            .json::<DistributionReleaseResponse>()
            .await
            .context("Failed to parse distribution release response")
    }

    pub async fn download_distribution_asset(&self, download_id: &str) -> Result<Vec<u8>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/download/{download_id}"))?;
        let response = self.client.get(url).send().await?.error_for_status()?;
        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_build_blob(&self, pack_id: &str, channel: &str) -> Result<BuildBlobResult> {
        let artifact = self.get_launcher_artifact(pack_id, channel, None).await?;
        let bytes = self.download_blob(&artifact.download_url).await?;

        Ok(BuildBlobResult {
            bytes,
            force_reinstall: artifact.force_reinstall.unwrap_or(false),
            requires_full_reinstall: artifact.requires_full_reinstall.unwrap_or(false),
            minecraft_version: artifact.minecraft_version,
            modloader: artifact.modloader,
            modloader_version: artifact.modloader_version,
        })
    }

    pub async fn list_pack_builds(&self, pack_id: &str) -> Result<Vec<PackBuild>> {
        let url = self.base_url.join(&format!("/api/v1/packs/{pack_id}/builds"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Debug, Deserialize)]
        struct BuildsResponse {
            builds: Vec<PackBuild>,
        }

        response
            .json::<BuildsResponse>()
            .await
            .context("Failed to parse pack builds response")
            .map(|payload| payload.builds)
    }

    pub async fn list_pack_channels(&self, pack_id: &str) -> Result<Vec<PackChannel>> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{pack_id}/channels"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        #[derive(Debug, Deserialize)]
        struct ChannelsResponse {
            channels: Vec<PackChannel>,
        }

        response
            .json::<ChannelsResponse>()
            .await
            .context("Failed to parse pack channels response")
            .map(|payload| payload.channels)
    }

    pub async fn promote_pack_channel(
        &self,
        pack_id: &str,
        channel: &str,
        build_id: &str,
    ) -> Result<()> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{pack_id}/channels"))?;
        let response = self
            .client
            .post(url)
            .headers(self.get_auth_headers().await?)
            .json(&serde_json::json!({
                "channel": channel,
                "buildId": build_id,
            }))
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Channel promotion failed (HTTP {}): {}",
                status.as_u16(),
                body
            );
        }
        Ok(())
    }

    pub async fn get_whitelist(&self, pack_id: &str) -> Result<Vec<WhitelistEntry>> {
        let (whitelist, _) = self.get_whitelist_with_version(pack_id, None).await?;
        Ok(whitelist)
    }

    pub async fn get_pack_metadata_with_etag(
        &self,
        pack_id: &str,
        channel: &str,
        etag: Option<&str>,
    ) -> Result<(Option<PackMetadataResponse>, String)> {
        let mut url = self
            .base_url
            .join(&format!("/api/v1/runner/packs/{pack_id}/metadata"))?;
        url.query_pairs_mut().append_pair("channel", channel);

        let mut request = self.client.get(url).headers(self.get_auth_headers().await?);

        if let Some(etag) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            // Prefer header if present; otherwise reuse the caller-provided etag.
            let returned = response
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string());

            let effective = returned
                .or_else(|| etag.map(|s| s.to_string()))
                .ok_or_else(|| {
                    anyhow::anyhow!("server returned 304 but no etag was provided or returned")
                })?;

            return Ok((None, effective));
        }

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Pack metadata request failed (HTTP {}): {}",
                status.as_u16(),
                body
            );
        }

        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| anyhow::anyhow!("metadata response missing ETag header"))?
            .to_string();

        let metadata: PackMetadataResponse = response.json().await?;
        Ok((Some(metadata), etag))
    }

    pub async fn get_whitelist_with_version(
        &self,
        pack_id: &str,
        etag: Option<&str>,
    ) -> Result<(Vec<WhitelistEntry>, String)> {
        let url = self
            .base_url
            .join(&format!("/api/v1/runner/packs/{pack_id}/whitelist"))?;
        let mut request = self.client.get(url).headers(self.get_auth_headers().await?);

        if let Some(etag) = etag {
            request = request.header("if-none-match", etag);
        }

        let response = request.send().await?;

        if response.status() == 304 {
            // Not modified, return empty vec with the same etag
            let etag = response
                .headers()
                .get("etag")
                .and_then(|h| h.to_str().ok())
                .unwrap_or(etag.unwrap_or(""));
            return Ok((Vec::new(), etag.to_string()));
        }

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Whitelist request failed (HTTP {}): {}",
                status.as_u16(),
                body
            );
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .to_string();

        let whitelist: Vec<WhitelistEntry> = response.json().await?;
        Ok((whitelist, etag))
    }

    pub async fn open_pack_update_events(&self, pack_id: &str) -> Result<Response> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{pack_id}/updates/stream"))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        Ok(response)
    }

    pub async fn login(&self) -> Result<DeviceCodeResponse> {
        let url = hub_device_code_endpoint(self.base_url.as_str());
        let request = DeviceCodeRequest {
            client_id: DEFAULT_ATLAS_DEVICE_CLIENT_ID,
            scope: DEFAULT_ATLAS_DEVICE_SCOPE,
        };

        let response = self
            .client
            .post(url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn poll_token(&mut self, device_code: &str) -> Result<Option<String>> {
        let url = hub_device_token_endpoint(self.base_url.as_str());
        let request = DeviceTokenRequest::new(DEFAULT_ATLAS_DEVICE_CLIENT_ID, device_code);

        let response = self.client.post(url).json(&request).send().await?;
        let status = response.status().as_u16();
        let body = response.text().await?;

        let poll_status: DeviceTokenPollStatus<StandardDeviceTokenResponse> =
            parse_device_token_poll_body(status, &body)?;

        match poll_status {
            DeviceTokenPollStatus::Success(token) => {
                self.set_token(token.access_token.clone());
                Ok(Some(token.access_token))
            }
            DeviceTokenPollStatus::AuthorizationPending | DeviceTokenPollStatus::SlowDown => {
                Ok(None)
            }
            DeviceTokenPollStatus::ExpiredToken => anyhow::bail!("Device code expired"),
            DeviceTokenPollStatus::AccessDenied => anyhow::bail!("Access denied"),
            DeviceTokenPollStatus::Fatal(err) => anyhow::bail!("Authentication failed: {}", err),
        }
    }

    pub async fn create_launcher_link_session(&self) -> Result<LauncherLinkSession> {
        let url = self.base_url.join("/api/v1/launcher/link-sessions")?;
        let response = self
            .client
            .post(url)
            .json(&serde_json::json!({}))
            .send()
            .await?
            .error_for_status()?;
        response
            .json::<LauncherLinkSession>()
            .await
            .context("Failed to parse launcher link session")
    }

    pub async fn complete_launcher_link_session(
        &self,
        payload: &LauncherLinkCompleteRequest,
    ) -> Result<LauncherLinkComplete> {
        let url = self
            .base_url
            .join("/api/v1/launcher/link-sessions/complete")?;
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await?
            .error_for_status()?;
        response
            .json::<LauncherLinkComplete>()
            .await
            .context("Failed to parse launcher link completion")
    }

    pub async fn get_mojang_info(&self, access_token: &str) -> Result<MojangInfoResponse> {
        let url = self.base_url.join("/api/v1/user/mojang/info")?;
        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?;
        response
            .json::<MojangInfoResponse>()
            .await
            .context("Failed to parse Mojang info")
    }

    /// Downloads the CI workflow template from the Atlas Hub.
    ///
    /// This endpoint fetches the GitHub Actions workflow template that should be used
    /// for CI/CD builds. The workflow is served from the configured template repository.
    ///
    /// # Returns
    ///
    /// Returns a `CiWorkflowResponse` containing the workflow file content and path.
    pub async fn download_ci_workflow(&self) -> Result<CiWorkflowResponse> {
        let url = self.base_url.join("/download/ci/workflow")?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        let workflow_path = response
            .headers()
            .get("x-atlas-workflow-path")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(".github/workflows/atlas-build.yml")
            .to_string();

        let content = response
            .text()
            .await
            .context("Failed to read CI workflow download response")?;

        Ok(CiWorkflowResponse {
            workflow_path,
            content,
        })
    }

    /// Retrieves the linked GitHub access token for the authenticated user.
    ///
    /// This endpoint returns the GitHub OAuth access token that was linked to the user's
    /// Atlas account. This token can be used to authenticate GitHub API requests on behalf
    /// of the user.
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` containing the GitHub access token if the user has linked
    /// their GitHub account, or `None` if no GitHub account is linked.
    pub async fn get_github_token(&self) -> Result<Option<String>> {
        let url = self.base_url.join("/api/v1/launcher/github/token")?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?;

        let status = response.status().as_u16();
        if status == 404 || status == 409 {
            return Ok(None);
        }
        if !(200..300).contains(&status) {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to fetch linked GitHub credentials (HTTP {}): {}",
                status,
                body
            );
        }

        let payload: GithubTokenResponse = response.json().await?;
        Ok(Some(payload.access_token))
    }

    /// Generates a presigned URL for CI artifact uploads.
    ///
    /// This endpoint creates a unique build ID and returns presigned upload URLs
    /// that can be used to upload build artifacts to storage. The build will be
    /// associated with the specified pack.
    ///
    /// # Parameters
    ///
    /// * `pack_id` - The ID of the pack for which the build artifacts will be uploaded
    ///
    /// # Returns
    ///
    /// Returns a `CiPresignResponse` containing the build ID, artifact key, and upload URL.
    pub async fn presign_ci_upload(&self, pack_id: &str) -> Result<CiPresignResponse> {
        let url = self.base_url.join("/api/v1/ci/presign")?;
        let mut request = self
            .client
            .post(url)
            .headers(self.get_auth_headers().await?)
            .json(&CiPresignRequest {
                pack_id: pack_id.to_string(),
            });
        if let Some(token) = self.get_pack_deploy_token() {
            request = request.header("x-atlas-pack-deploy-token", token);
        }
        let response = request.send().await?.error_for_status()?;

        response
            .json::<CiPresignResponse>()
            .await
            .context("Failed to parse CI presign response")
    }

    /// Completes a CI build and updates pack channels.
    ///
    /// This endpoint finalizes a CI build by storing build metadata and updating
    /// the specified pack channel to point to the new build. The build artifact
    /// must have been previously uploaded using the presigned URL from `presign_ci_upload`.
    ///
    /// # Parameters
    ///
    /// * `request` - The completion request containing build metadata and channel information
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful completion.
    pub async fn complete_ci_build(&self, request: &CiCompleteRequest) -> Result<()> {
        let url = self.base_url.join("/api/v1/ci/complete")?;
        let mut req = self
            .client
            .post(url)
            .headers(self.get_auth_headers().await?)
            .json(request);
        if let Some(token) = self.get_pack_deploy_token() {
            req = req.header("x-atlas-pack-deploy-token", token);
        }
        req.send().await?.error_for_status()?;
        Ok(())
    }

    fn block_on_hub_future<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::MultiThread => {
                    tokio::task::block_in_place(|| handle.block_on(future))
                }
                tokio::runtime::RuntimeFlavor::CurrentThread => Err(anyhow::anyhow!(
                    "HubClient blocking_* methods cannot run inside a current-thread Tokio runtime."
                )),
                _ => tokio::task::block_in_place(|| handle.block_on(future)),
            };
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to create Tokio runtime for blocking Hub client call")?;
        runtime.block_on(future)
    }

    // Blocking versions for synchronous CLI usage

    /// Downloads the CI workflow template (blocking version).
    ///
    /// Synchronous wrapper for `download_ci_workflow()` that blocks the current thread
    /// until the operation completes. Useful for synchronous CLI applications.
    ///
    /// # Returns
    ///
    /// Returns a `CiWorkflowResponse` containing the workflow file content and path.
    pub fn blocking_download_ci_workflow(&self) -> Result<CiWorkflowResponse> {
        self.block_on_hub_future(self.download_ci_workflow())
    }

    /// Retrieves the linked GitHub access token (blocking version).
    ///
    /// Synchronous wrapper for `get_github_token()` that blocks the current thread
    /// until the operation completes. Useful for synchronous CLI applications.
    ///
    /// # Returns
    ///
    /// Returns `Some(String)` containing the GitHub access token if the user has linked
    /// their GitHub account, or `None` if no GitHub account is linked.
    pub fn blocking_get_github_token(&self) -> Result<Option<String>> {
        self.block_on_hub_future(self.get_github_token())
    }

    /// Generates a presigned URL for CI artifact uploads (blocking version).
    ///
    /// Synchronous wrapper for `presign_ci_upload()` that blocks the current thread
    /// until the operation completes. Useful for synchronous CLI applications.
    ///
    /// # Parameters
    ///
    /// * `pack_id` - The ID of the pack for which the build artifacts will be uploaded
    ///
    /// # Returns
    ///
    /// Returns a `CiPresignResponse` containing the build ID, artifact key, and upload URL.
    pub fn blocking_presign_ci_upload(&self, pack_id: &str) -> Result<CiPresignResponse> {
        self.block_on_hub_future(self.presign_ci_upload(pack_id))
    }

    /// Completes a CI build (blocking version).
    ///
    /// Synchronous wrapper for `complete_ci_build()` that blocks the current thread
    /// until the operation completes. Useful for synchronous CLI applications.
    ///
    /// # Parameters
    ///
    /// * `request` - The completion request containing build metadata and channel information
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful completion.
    pub fn blocking_complete_ci_build(&self, request: &CiCompleteRequest) -> Result<()> {
        self.block_on_hub_future(self.complete_ci_build(request))
    }

    /// Lists launcher packs (blocking version).
    ///
    /// Synchronous wrapper for `list_launcher_packs()` that blocks the current thread
    /// until the operation completes. Useful for synchronous CLI applications.
    ///
    /// # Returns
    ///
    /// Returns a vector of `LauncherPack` objects representing available packs.
    pub fn blocking_list_launcher_packs(&self) -> Result<Vec<LauncherPack>> {
        self.block_on_hub_future(self.list_launcher_packs())
    }

    pub fn blocking_list_pack_builds(&self, pack_id: &str) -> Result<Vec<PackBuild>> {
        self.block_on_hub_future(self.list_pack_builds(pack_id))
    }

    pub fn blocking_list_pack_channels(&self, pack_id: &str) -> Result<Vec<PackChannel>> {
        self.block_on_hub_future(self.list_pack_channels(pack_id))
    }

    pub fn blocking_promote_pack_channel(
        &self,
        pack_id: &str,
        channel: &str,
        build_id: &str,
    ) -> Result<()> {
        self.block_on_hub_future(self.promote_pack_channel(pack_id, channel, build_id))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLinkCompleteRequest {
    pub link_session_id: String,
    pub proof: String,
    pub minecraft: LauncherMinecraftPayload,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherMinecraftPayload {
    pub uuid: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WhitelistEntry {
    pub uuid: String,
    pub name: String,
}

/// Response from downloading a CI workflow template.
#[derive(Debug, Serialize, Deserialize)]
pub struct CiWorkflowResponse {
    /// The recommended path where the workflow file should be placed in the repository
    pub workflow_path: String,
    /// The YAML content of the workflow file
    pub content: String,
}

/// Response containing a GitHub access token.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubTokenResponse {
    /// The GitHub OAuth access token
    pub access_token: String,
}

/// Request for presigning a CI artifact upload.
#[derive(Debug, Serialize)]
pub struct CiPresignRequest {
    /// The ID of the pack for which artifacts will be uploaded
    #[serde(rename = "packId")]
    pub pack_id: String,
}

/// Response from presigning a CI artifact upload.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiPresignResponse {
    /// Unique identifier for this build
    pub build_id: String,
    /// Encoded key for the artifact in storage
    pub artifact_key: String,
    /// Presigned URL for uploading the artifact
    pub upload_url: String,
    /// Optional headers required by the storage provider for direct uploads
    #[serde(default)]
    pub upload_headers: std::collections::HashMap<String, String>,
}

/// Request to complete a CI build.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CiCompleteRequest {
    /// The ID of the pack this build belongs to
    pub pack_id: String,
    /// The build ID returned from the presign request
    pub build_id: String,
    /// The artifact key returned from the presign request
    pub artifact_key: String,
    /// Version string for this build
    pub version: String,
    /// Optional Git commit hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_hash: Option<String>,
    /// Optional Git commit message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_message: Option<String>,
    /// Optional Minecraft version this build targets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minecraft_version: Option<String>,
    /// Optional modloader type (e.g., "fabric", "forge")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modloader: Option<String>,
    /// Optional modloader version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modloader_version: Option<String>,
    /// Size of the uploaded artifact in bytes
    pub artifact_size: u64,
    /// Channel to update with this build ("dev", "beta", or "production")
    pub channel: String,
}
