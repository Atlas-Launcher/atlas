use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use url::Url;
use atlas_auth::device_code::DeviceCodeResponse;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub mod whitelist;

pub struct HubClient {
    client: Client,
    base_url: Url,
    auth: Mutex<AuthState>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPermission {
    pub pack_id: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherPack {
    pub pack_id: String,
    pub pack_name: String,
    pub pack_slug: String,
    pub channel: String,
}

#[derive(Debug, Deserialize)]
struct LauncherPacksResponse {
    packs: Vec<LauncherPack>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherArtifactResponse {
    download_url: String,
    force_reinstall: Option<bool>,
    requires_full_reinstall: Option<bool>,
    minecraft_version: Option<String>,
    modloader: Option<String>,
}

#[derive(Debug)]
pub struct BuildBlobResult {
    pub bytes: Vec<u8>,
    pub force_reinstall: bool,
    pub requires_full_reinstall: bool,
    pub minecraft_version: Option<String>,
    pub modloader: Option<String>,
}

impl HubClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        Ok(Self {
            client: Client::new(),
            base_url,
            auth: Mutex::new(AuthState::None),
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

    async fn get_auth_headers(&self) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        let auth = { self.auth.lock().expect("auth lock poisoned").clone() };
        match auth {
            AuthState::None => {
                return Ok(headers);
            }
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

    async fn ensure_service_access_token(
        &self,
        mut state: ServiceAuthState,
    ) -> Result<String> {
        if let Some(access) = &state.access_token {
            if Instant::now() + Duration::from_secs(60) < access.expires_at {
                return Ok(access.value.clone());
            }
        }

        let exchange = self.exchange_service_token(&state.service_token).await?;
        let access = AccessToken {
            value: exchange.access_token.clone(),
            expires_at: Instant::now()
                + Duration::from_secs(exchange.expires_in.max(30)),
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
            .await?
            .error_for_status()?;

        response
            .json()
            .await
            .context("Failed to parse runner token exchange")
    }

    pub async fn validate_service_token(&self) -> Result<RunnerTokenExchange> {
        let auth = { self.auth.lock().expect("auth lock poisoned").clone() };
        match auth {
            AuthState::ServiceToken(state) => self.exchange_service_token(&state.service_token).await,
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

        response
            .json()
            .await
            .context("Failed to parse runner service token")
    }

    pub async fn get_pack_metadata(&self, pack_id: &str) -> Result<PackMetadata> {
        let url = self.base_url.join(&format!("/api/v1/packs/{}", pack_id))?;
        let response = self.client.get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;
        
        response.json().await.context("Failed to parse pack metadata")
    }

    pub async fn list_launcher_packs(&self) -> Result<Vec<LauncherPack>> {
        let url = self.base_url.join("/api/launcher/packs")?;
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
        let url = self.base_url.join(&format!("/api/v1/packs/{}/check-access", pack_id))?;
        let response = self.client.get(url)
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

    pub async fn get_build_blob(&self, pack_id: &str, channel: &str) -> Result<BuildBlobResult> {
        let mut url = self
            .base_url
            .join(&format!("/api/launcher/packs/{}/artifact", pack_id))?;
        url.query_pairs_mut().append_pair("channel", channel);

        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        let artifact: LauncherArtifactResponse = response
            .json()
            .await
            .context("Failed to parse launcher artifact response")?;

        let download = self
            .client
            .get(artifact.download_url)
            .send()
            .await?
            .error_for_status()?;

        Ok(BuildBlobResult {
            bytes: download.bytes().await?.to_vec(),
            force_reinstall: artifact.force_reinstall.unwrap_or(false),
            requires_full_reinstall: artifact.requires_full_reinstall.unwrap_or(false),
            minecraft_version: artifact.minecraft_version,
            modloader: artifact.modloader,
        })
    }

    pub async fn get_whitelist(
        &self,
        pack_id: &str,
    ) -> Result<Vec<whitelist::WhitelistEntry>> {
        let url = self.base_url.join(&format!("/api/v1/packs/{}/whitelist", pack_id))?;
        let response = self.client.get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response.json().await.context("Failed to parse whitelist")
    }

    pub async fn open_whitelist_events(&self, pack_id: &str) -> Result<reqwest::Response> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{}/whitelist/stream", pack_id))?;
        let response = self
            .client
            .get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        Ok(response)
    }

    pub async fn open_pack_update_events(&self, pack_id: &str) -> Result<reqwest::Response> {
        let url = self
            .base_url
            .join(&format!("/api/v1/packs/{}/updates/stream", pack_id))?;
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
        let url = atlas_auth::device_code::hub_device_code_endpoint(self.base_url.as_str());
        let request = atlas_auth::device_code::DeviceCodeRequest {
            client_id: atlas_auth::device_code::DEFAULT_ATLAS_DEVICE_CLIENT_ID,
            scope: atlas_auth::device_code::DEFAULT_ATLAS_DEVICE_SCOPE,
        };

        let response = self.client.post(url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    pub async fn poll_token(&mut self, device_code: &str) -> Result<Option<String>> {
        let url = atlas_auth::device_code::hub_device_token_endpoint(self.base_url.as_str());
        let request = atlas_auth::device_code::DeviceTokenRequest::new(
            atlas_auth::device_code::DEFAULT_ATLAS_DEVICE_CLIENT_ID,
            device_code,
        );

        let response = self.client.post(url)
            .json(&request)
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = response.text().await?;
        
        use atlas_auth::device_code::{DeviceTokenPollStatus, StandardDeviceTokenResponse, parse_device_token_poll_body};
        let poll_status: DeviceTokenPollStatus<StandardDeviceTokenResponse> = parse_device_token_poll_body(status, &body)?;

        match poll_status {
            DeviceTokenPollStatus::Success(token) => {
                self.set_token(token.access_token.clone());
                Ok(Some(token.access_token))
            }
            DeviceTokenPollStatus::AuthorizationPending | DeviceTokenPollStatus::SlowDown => Ok(None),
            DeviceTokenPollStatus::ExpiredToken => anyhow::bail!("Device code expired"),
            DeviceTokenPollStatus::AccessDenied => anyhow::bail!("Access denied"),
            DeviceTokenPollStatus::Fatal(err) => anyhow::bail!("Authentication failed: {}", err),
        }
    }
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
    pub id: Option<String>,
    pub pack_id: String,
    pub token: String,
    pub prefix: String,
}
