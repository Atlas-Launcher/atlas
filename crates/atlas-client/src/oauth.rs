use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

const AUTH_SCOPE: &str = "openid profile email offline_access";

#[derive(Debug, Deserialize, Clone)]
pub struct AtlasTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AtlasUserInfo {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub mojang_username: Option<String>,
    #[serde(default)]
    pub mojang_uuid: Option<String>,
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

pub struct AuthRequest {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
}

pub fn build_auth_request(
    auth_base_url: &str,
    client_id: &str,
    redirect_uri: &str,
) -> Result<AuthRequest> {
    let state = random_url_safe(16);
    let code_verifier = random_url_safe(64);
    let code_challenge = code_challenge_s256(&code_verifier);

    let mut url = Url::parse(&format!(
        "{}/oauth2/authorize",
        auth_base_url.trim_end_matches('/')
    ))
    .context("Invalid Atlas authorization URL")?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", AUTH_SCOPE)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("prompt", "login");

    Ok(AuthRequest {
        auth_url: url.to_string(),
        state,
        code_verifier,
    })
}

pub async fn exchange_auth_code(
    auth_base_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<AtlasTokenResponse> {
    let token_url = format!("{}/oauth2/token", auth_base_url.trim_end_matches('/'));
    let params = [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    let response = reqwest::Client::new()
        .post(token_url)
        .form(&params)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

pub async fn refresh_token(
    auth_base_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<AtlasTokenResponse> {
    let token_url = format!("{}/oauth2/token", auth_base_url.trim_end_matches('/'));
    let params = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = reqwest::Client::new()
        .post(token_url)
        .form(&params)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

pub async fn fetch_user_info(auth_base_url: &str, access_token: &str) -> Result<AtlasUserInfo> {
    let user_info_url = format!("{}/oauth2/userinfo", auth_base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(user_info_url)
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;

    Ok(response.json().await?)
}

pub fn parse_auth_callback(callback_url: &str, expected_state: &str) -> Result<String> {
    let url = Url::parse(callback_url).context("Invalid auth callback URL")?;
    parse_auth_callback_url(&url, expected_state)
}

fn parse_auth_callback_url(url: &Url, expected_state: &str) -> Result<String> {
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
        anyhow::bail!("Atlas sign-in failed: {error}{description}");
    }

    if params.get("state").map(String::as_str) != Some(expected_state) {
        anyhow::bail!("Sign-in state did not match. Please try again.");
    }

    Ok(params
        .get("code")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Missing authorization code in redirect."))?)
}

fn parse_pairs(raw: &str, params: &mut HashMap<String, String>) {
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        params.entry(key.into()).or_insert(value.into());
    }
}
