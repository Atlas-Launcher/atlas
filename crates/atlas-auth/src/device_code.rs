use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const DEFAULT_ATLAS_HUB_URL: &str = "https://atlas.nathanm.org";
pub const DEFAULT_ATLAS_DEVICE_CLIENT_ID: &str = "atlas-launcher";
pub const DEFAULT_ATLAS_DEVICE_SCOPE: &str = "openid profile email offline_access";
pub const DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Debug, Serialize)]
pub struct DeviceCodeRequest<'a> {
    pub client_id: &'a str,
    pub scope: &'a str,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceTokenRequest<'a> {
    pub grant_type: &'a str,
    pub device_code: &'a str,
    pub client_id: &'a str,
}

impl<'a> DeviceTokenRequest<'a> {
    pub fn new(client_id: &'a str, device_code: &'a str) -> Self {
        Self {
            grant_type: DEVICE_CODE_GRANT_TYPE,
            device_code,
            client_id,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OAuthDeviceTokenError {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StandardDeviceTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DeviceTokenPollStatus<T> {
    Success(T),
    AuthorizationPending,
    SlowDown,
    ExpiredToken,
    AccessDenied,
    Fatal(String),
}

#[derive(Debug, Error)]
pub enum DeviceCodeParseError {
    #[error("Failed to parse OAuth2 device token response: {0}")]
    Parse(#[from] serde_json::Error),
}

pub fn normalize_hub_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

pub fn hub_device_code_endpoint(hub_url: &str) -> String {
    format!("{}/api/auth/device/code", normalize_hub_url(hub_url))
}

pub fn hub_device_token_endpoint(hub_url: &str) -> String {
    format!("{}/api/auth/device/token", normalize_hub_url(hub_url))
}

pub fn parse_device_token_poll_json<T: DeserializeOwned>(
    value: Value,
) -> Result<DeviceTokenPollStatus<T>, DeviceCodeParseError> {
    if let Ok(success) = serde_json::from_value::<T>(value.clone()) {
        return Ok(DeviceTokenPollStatus::Success(success));
    }

    let err = serde_json::from_value::<OAuthDeviceTokenError>(value)?;
    Ok(map_device_token_error(err))
}

pub fn parse_device_token_poll_body<T: DeserializeOwned>(
    status: u16,
    body: &str,
) -> Result<DeviceTokenPollStatus<T>, DeviceCodeParseError> {
    if (200..300).contains(&status) {
        let success = serde_json::from_str::<T>(body)?;
        return Ok(DeviceTokenPollStatus::Success(success));
    }

    let err = serde_json::from_str::<OAuthDeviceTokenError>(body).unwrap_or(OAuthDeviceTokenError {
        error: "unknown".to_string(),
        error_description: Some(body.to_string()),
    });
    Ok(map_device_token_error(err))
}

fn map_device_token_error_unit(err: OAuthDeviceTokenError) -> DeviceTokenPollStatus<()> {
    match err.error.as_str() {
        "authorization_pending" => DeviceTokenPollStatus::AuthorizationPending,
        "slow_down" => DeviceTokenPollStatus::SlowDown,
        "expired_token" => DeviceTokenPollStatus::ExpiredToken,
        "access_denied" => DeviceTokenPollStatus::AccessDenied,
        _ => DeviceTokenPollStatus::Fatal(err.error_description.unwrap_or(err.error)),
    }
}

fn map_device_token_error<T>(err: OAuthDeviceTokenError) -> DeviceTokenPollStatus<T> {
    match map_device_token_error_unit(err) {
        DeviceTokenPollStatus::AuthorizationPending => DeviceTokenPollStatus::AuthorizationPending,
        DeviceTokenPollStatus::SlowDown => DeviceTokenPollStatus::SlowDown,
        DeviceTokenPollStatus::ExpiredToken => DeviceTokenPollStatus::ExpiredToken,
        DeviceTokenPollStatus::AccessDenied => DeviceTokenPollStatus::AccessDenied,
        DeviceTokenPollStatus::Fatal(message) => DeviceTokenPollStatus::Fatal(message),
        DeviceTokenPollStatus::Success(_) => unreachable!(),
    }
}
