use crate::models::Profile;
use serde::Deserialize;
use serde_json::json;

use super::error::AuthError;
use crate::net::http::HttpClient;

const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Deserialize)]
pub struct MinecraftLoginResponse {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
    items: Vec<serde_json::Value>,
}

pub async fn login<H: HttpClient + ?Sized>(
    http: &H,
    xsts_token: &str,
    uhs: &str,
) -> Result<MinecraftLoginResponse, AuthError> {
    let body = json!({
      "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token)
    });

    Ok(http.post_json(MC_LOGIN_URL, &body).await?)
}

pub async fn profile<H: HttpClient + ?Sized>(
    http: &H,
    access_token: &str,
) -> Result<Profile, AuthError> {
    Ok(http.get_json(MC_PROFILE_URL, Some(access_token)).await?)
}

pub async fn verify_entitlements<H: HttpClient + ?Sized>(
    http: &H,
    access_token: &str,
) -> Result<(), AuthError> {
    let entitlements: EntitlementsResponse = http
        .get_json(MC_ENTITLEMENTS_URL, Some(access_token))
        .await?;

    if entitlements.items.is_empty() {
        return Err(AuthError::MissingMinecraftEntitlement);
    }

    Ok(())
}
