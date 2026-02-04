use crate::models::DeviceCodeResponse;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use url::Url;

use super::http::HttpClient;
const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    #[allow(dead_code)]
    pub expires_in: u64,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub scope: String,
}

#[derive(Debug, Deserialize)]
struct DeviceTokenError {
    error: String,
    #[allow(dead_code)]
    error_description: Option<String>,
}

pub async fn start_device_code<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
) -> Result<DeviceCodeResponse, String> {
    let params = [
        ("client_id", client_id),
        ("scope", "XboxLive.signin offline_access"),
    ];

    http.post_form(DEVICE_CODE_URL, &params).await
}

pub(crate) async fn poll_device_token<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    device_code: &str,
) -> Result<DeviceTokenResponse, String> {
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
            ("device_code", device_code),
        ];

        let response = http
            .post_form::<serde_json::Value>(TOKEN_URL, &params)
            .await?;

        if let Ok(token) = serde_json::from_value::<DeviceTokenResponse>(response.clone()) {
            return Ok(token);
        }

        let error =
            serde_json::from_value::<DeviceTokenError>(response).unwrap_or(DeviceTokenError {
                error: "unknown".into(),
                error_description: None,
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

pub(crate) async fn refresh_token<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    refresh_token: &str,
) -> Result<DeviceTokenResponse, String> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    http.post_form(TOKEN_URL, &params).await
}

fn random_url_safe(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}

fn code_challenge_s256(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

pub(crate) async fn exchange_auth_code<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<DeviceTokenResponse, String> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    http.post_form(TOKEN_URL, &params).await
}

pub(crate) struct AuthRequest {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
}

pub(crate) fn build_auth_request(
    client_id: &str,
    redirect_uri: &str,
) -> Result<AuthRequest, String> {
    let state = random_url_safe(16);
    let code_verifier = random_url_safe(64);
    let code_challenge = code_challenge_s256(&code_verifier);

    let mut url =
        Url::parse(AUTHORIZE_URL).map_err(|err| format!("Invalid authorize URL: {err}"))?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", "XboxLive.signin offline_access")
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("prompt", "select_account");

    Ok(AuthRequest {
        auth_url: url.to_string(),
        state,
        code_verifier,
    })
}

pub(crate) fn parse_auth_callback(
    callback_url: &str,
    expected_state: &str,
) -> Result<String, String> {
    let url =
        Url::parse(callback_url).map_err(|err| format!("Invalid auth callback URL: {err}"))?;
    parse_auth_callback_url(&url, expected_state)
}

fn parse_auth_callback_url(url: &Url, expected_state: &str) -> Result<String, String> {
    let mut params = HashMap::new();
    if let Some(query) = url.query() {
        parse_pairs(query, &mut params);
    }
    if let Some(fragment) = url.fragment() {
        parse_pairs(fragment, &mut params);
    }

    if let Some(error) = params.get("error") {
        let description = params
            .get("error_description")
            .map(|value| format!(" ({value})"))
            .unwrap_or_default();
        return Err(format!("Microsoft sign-in failed: {error}{description}"));
    }

    if params.get("state").map(String::as_str) != Some(expected_state) {
        return Err("Sign-in state did not match. Please try again.".to_string());
    }

    params
        .get("code")
        .cloned()
        .ok_or_else(|| "Missing authorization code in redirect.".to_string())
}

fn parse_pairs(raw: &str, params: &mut HashMap<String, String>) {
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        params.entry(key.into()).or_insert(value.into());
    }
}
