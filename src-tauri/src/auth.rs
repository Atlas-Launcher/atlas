use crate::models::{AuthSession, DeviceCodeResponse, Profile};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::sleep;

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Deserialize)]
struct DeviceTokenResponse {
  access_token: String,
  #[allow(dead_code)]
  refresh_token: Option<String>,
  #[allow(dead_code)]
  expires_in: u64,
  #[allow(dead_code)]
  token_type: String,
  #[allow(dead_code)]
  scope: String
}

#[derive(Debug, Deserialize)]
struct DeviceTokenError {
  error: String,
  #[allow(dead_code)]
  error_description: Option<String>
}

#[derive(Debug, Deserialize)]
struct XboxAuthResponse {
  Token: String,
  DisplayClaims: XboxDisplayClaims
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
  xui: Vec<XboxUserClaim>
}

#[derive(Debug, Deserialize)]
struct XboxUserClaim {
  uhs: String
}

#[derive(Debug, Deserialize)]
struct MinecraftLoginResponse {
  access_token: String,
  #[allow(dead_code)]
  expires_in: u64
}

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
  items: Vec<serde_json::Value>
}

pub async fn start_device_code(client_id: &str) -> Result<DeviceCodeResponse, String> {
  let client = Client::new();
  let params = [
    ("client_id", client_id),
    ("scope", "XboxLive.signin offline_access")
  ];

  let response = client
    .post(DEVICE_CODE_URL)
    .form(&params)
    .send()
    .await
    .map_err(|err| format!("Device code request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Device code request failed ({status}): {text}"));
  }

  response
    .json::<DeviceCodeResponse>()
    .await
    .map_err(|err| format!("Failed to parse device code response: {err}"))
}

pub async fn complete_device_code(client_id: &str, device_code: &str) -> Result<AuthSession, String> {
  let token = poll_device_token(client_id, device_code).await?;
  let xbl = xbox_authenticate(&token.access_token).await?;
  let xsts = xbox_xsts(&xbl.Token).await?;
  let uhs = xsts
    .DisplayClaims
    .xui
    .get(0)
    .map(|claim| claim.uhs.clone())
    .ok_or_else(|| "Missing Xbox user hash".to_string())?;
  let mc = minecraft_login(&xsts.Token, &uhs).await?;

  let profile = minecraft_profile(&mc.access_token).await?;
  verify_entitlements(&mc.access_token).await?;

  Ok(AuthSession {
    access_token: mc.access_token,
    profile
  })
}

async fn poll_device_token(client_id: &str, device_code: &str) -> Result<DeviceTokenResponse, String> {
  let client = Client::new();
  let mut interval = Duration::from_secs(5);
  let start = Instant::now();
  let timeout = Duration::from_secs(900);

  loop {
    if start.elapsed() > timeout {
      return Err("Device code expired. Start login again.".into());
    }

    let params = [
      ("client_id", client_id),
      ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
      ("device_code", device_code)
    ];

    let response = client
      .post(TOKEN_URL)
      .form(&params)
      .send()
      .await
      .map_err(|err| format!("Token polling failed: {err}"))?;

    if response.status().is_success() {
      return response
        .json::<DeviceTokenResponse>()
        .await
        .map_err(|err| format!("Failed to parse token response: {err}"));
    }

    let error = response
      .json::<DeviceTokenError>()
      .await
      .unwrap_or(DeviceTokenError {
        error: "unknown".into(),
        error_description: None
      });

    match error.error.as_str() {
      "authorization_pending" => {
        sleep(interval).await;
      }
      "slow_down" => {
        interval += Duration::from_secs(5);
        sleep(interval).await;
      }
      "expired_token" => return Err("Device code expired. Start login again.".into()),
      _ => {
        return Err(format!(
          "Device code login failed: {}",
          error.error_description.unwrap_or(error.error)
        ));
      }
    }
  }
}

async fn xbox_authenticate(ms_access_token: &str) -> Result<XboxAuthResponse, String> {
  let client = Client::new();
  let body = json!({
    "Properties": {
      "AuthMethod": "RPS",
      "SiteName": "user.auth.xboxlive.com",
      "RpsTicket": format!("d={}", ms_access_token)
    },
    "RelyingParty": "http://auth.xboxlive.com",
    "TokenType": "JWT"
  });

  post_json(&client, XBL_AUTH_URL, &body).await
}

async fn xbox_xsts(xbl_token: &str) -> Result<XboxAuthResponse, String> {
  let client = Client::new();
  let body = json!({
    "Properties": {
      "SandboxId": "RETAIL",
      "UserTokens": [xbl_token]
    },
    "RelyingParty": "rp://api.minecraftservices.com/",
    "TokenType": "JWT"
  });

  post_json(&client, XSTS_AUTH_URL, &body).await
}

async fn minecraft_login(xsts_token: &str, uhs: &str) -> Result<MinecraftLoginResponse, String> {
  let client = Client::new();
  let body = json!({
    "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
  });

  post_json(&client, MC_LOGIN_URL, &body).await
}

async fn minecraft_profile(access_token: &str) -> Result<Profile, String> {
  let client = Client::new();
  let response = client
    .get(MC_PROFILE_URL)
    .bearer_auth(access_token)
    .send()
    .await
    .map_err(|err| format!("Profile request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Profile request failed ({status}): {text}"));
  }

  response
    .json::<Profile>()
    .await
    .map_err(|err| format!("Failed to parse profile response: {err}"))
}

async fn verify_entitlements(access_token: &str) -> Result<(), String> {
  let client = Client::new();
  let response = client
    .get(MC_ENTITLEMENTS_URL)
    .bearer_auth(access_token)
    .send()
    .await
    .map_err(|err| format!("Entitlements request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Entitlements request failed ({status}): {text}"));
  }

  let entitlements = response
    .json::<EntitlementsResponse>()
    .await
    .map_err(|err| format!("Failed to parse entitlements: {err}"))?;

  if entitlements.items.is_empty() {
    return Err("Minecraft entitlement not found for this account.".into());
  }

  Ok(())
}

async fn post_json<T: DeserializeOwned, B: Serialize>(
  client: &Client,
  url: &str,
  body: &B
) -> Result<T, String> {
  let response = client
    .post(url)
    .json(body)
    .send()
    .await
    .map_err(|err| format!("Request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Request failed ({status}): {text}"));
  }

  response
    .json::<T>()
    .await
    .map_err(|err| format!("Failed to parse response: {err}"))
}
