use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

impl From<atlas_auth::device_code::DeviceCodeResponse> for DeviceCodeResponse {
    fn from(value: atlas_auth::device_code::DeviceCodeResponse) -> Self {
        Self {
            device_code: value.device_code,
            user_code: value.user_code,
            verification_uri: value.verification_uri,
            verification_uri_complete: value.verification_uri_complete,
            expires_in: value.expires_in,
            interval: value.interval,
            message: value.message,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AtlasProfile {
    pub id: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub mojang_username: Option<String>,
    #[serde(default)]
    pub mojang_uuid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthSession {
    pub access_token: String,
    pub profile: Profile,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: u64,
    #[serde(default)]
    pub client_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AtlasSession {
    pub access_token: String,
    pub profile: AtlasProfile,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub access_token_expires_at: u64,
    #[serde(default)]
    pub client_id: String,
    pub auth_base_url: String,
}
