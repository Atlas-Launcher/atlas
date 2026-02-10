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

impl From<atlas_client::device_code::DeviceCodeResponse> for DeviceCodeResponse {
    fn from(value: atlas_client::device_code::DeviceCodeResponse) -> Self {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLinkSession {
    pub link_session_id: String,
    pub link_code: String,
    pub proof: String,
    pub expires_at: String,
}

impl From<atlas_client::hub::LauncherLinkSession> for LauncherLinkSession {
    fn from(value: atlas_client::hub::LauncherLinkSession) -> Self {
        Self {
            link_session_id: value.link_session_id,
            link_code: value.link_code,
            proof: value.proof,
            expires_at: value.expires_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLinkComplete {
    pub success: bool,
    pub user_id: String,
    #[serde(default)]
    pub warning: Option<String>,
}

impl From<atlas_client::hub::LauncherLinkComplete> for LauncherLinkComplete {
    fn from(value: atlas_client::hub::LauncherLinkComplete) -> Self {
        Self {
            success: value.success,
            user_id: value.user_id,
            warning: value.warning,
        }
    }
}
