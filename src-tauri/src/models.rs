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
  pub message: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
  pub id: String,
  pub name: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthSession {
  pub access_token: String,
  pub profile: Profile
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchOptions {
  pub game_dir: String,
  pub java_path: String,
  pub memory_mb: u32,
  #[serde(default)]
  pub version: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchEvent {
  pub phase: String,
  pub message: String,
  #[serde(default)]
  pub current: Option<u64>,
  #[serde(default)]
  pub total: Option<u64>,
  #[serde(default)]
  pub percent: Option<u64>
}
