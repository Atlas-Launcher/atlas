use anyhow::{Context, Result};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use url::Url;
use atlas_auth::device_code::DeviceCodeResponse;

pub mod whitelist;

pub struct HubClient {
    client: Client,
    base_url: Url,
    token: Option<String>,
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

impl HubClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url)?;
        Ok(Self {
            client: Client::new(),
            base_url,
            token: None,
        })
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    async fn get_auth_headers(&self) -> Result<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        if let Some(token) = &self.token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
        }
        Ok(headers)
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

    pub async fn get_build_lockfile(&self, pack_id: &str, channel: &str) -> Result<Vec<u8>> {
        let url = self.base_url.join(&format!("/api/v1/packs/{}/channels/{}/lockfile", pack_id, channel))?;
        let response = self.client.get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_build_blob(&self, pack_id: &str, channel: &str) -> Result<Vec<u8>> {
        let url = self.base_url.join(&format!("/api/v1/packs/{}/channels/{}/blob", pack_id, channel))?;
        let response = self.client.get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn get_whitelist(&self, pack_id: &str) -> Result<Vec<String>> {
        let url = self.base_url.join(&format!("/api/v1/packs/{}/whitelist", pack_id))?;
        let response = self.client.get(url)
            .headers(self.get_auth_headers().await?)
            .send()
            .await?
            .error_for_status()?;

        response.json().await.context("Failed to parse whitelist")
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
