use crate::models::Profile;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::http::post_json;

const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Deserialize)]
pub struct MinecraftLoginResponse {
  pub access_token: String,
  pub expires_in: u64
}

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
  items: Vec<serde_json::Value>
}

pub async fn login(xsts_token: &str, uhs: &str) -> Result<MinecraftLoginResponse, String> {
  let client = Client::new();
  let body = json!({
    "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
  });

  post_json(&client, MC_LOGIN_URL, &body).await
}

pub async fn profile(access_token: &str) -> Result<Profile, String> {
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

pub async fn verify_entitlements(access_token: &str) -> Result<(), String> {
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
