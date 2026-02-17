use crate::models::DeviceCodeResponse;
use atlas_client::device_code::{
    parse_device_token_poll_json, DeviceTokenPollStatus, DEVICE_CODE_GRANT_TYPE,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use url::Url;

use super::error::AuthError;
use crate::net::http::HttpClient;
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

pub async fn start_device_code<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
) -> Result<DeviceCodeResponse, AuthError> {
    let params = [
        ("client_id", client_id),
        ("scope", "XboxLive.signin offline_access"),
    ];

    let response = http
        .post_form::<atlas_client::device_code::DeviceCodeResponse>(DEVICE_CODE_URL, &params)
        .await?;
    Ok(response.into())
}

pub(crate) async fn poll_device_token<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    device_code: &str,
) -> Result<DeviceTokenResponse, AuthError> {
    let mut interval = Duration::from_secs(5);
    let start = Instant::now();
    let timeout = Duration::from_secs(900);

    loop {
        if start.elapsed() > timeout {
            return Err("Device code expired. Start login again.".to_string().into());
        }

        let params = [
            ("client_id", client_id),
            ("grant_type", DEVICE_CODE_GRANT_TYPE),
            ("device_code", device_code),
        ];

        let response = http
            .post_form::<serde_json::Value>(TOKEN_URL, &params)
            .await?;
        let status = parse_device_token_poll_json::<DeviceTokenResponse>(response)
            .map_err(|err| err.to_string())?;

        match status {
            DeviceTokenPollStatus::Success(token) => return Ok(token),
            DeviceTokenPollStatus::AuthorizationPending => {
                sleep(interval).await;
            }
            DeviceTokenPollStatus::SlowDown => {
                interval += Duration::from_secs(5);
                sleep(interval).await;
            }
            DeviceTokenPollStatus::ExpiredToken => {
                return Err("Device code expired. Start login again.".to_string().into())
            }
            DeviceTokenPollStatus::AccessDenied => {
                return Err("Device code authorization denied.".to_string().into())
            }
            DeviceTokenPollStatus::Fatal(message) => {
                return Err(format!("Device code login failed: {message}").into())
            }
        }
    }
}

pub(crate) async fn refresh_token<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    refresh_token: &str,
) -> Result<DeviceTokenResponse, AuthError> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    Ok(http.post_form(TOKEN_URL, &params).await?)
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
) -> Result<DeviceTokenResponse, AuthError> {
    let params = [
        ("client_id", client_id),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("code_verifier", code_verifier),
    ];

    Ok(http.post_form(TOKEN_URL, &params).await?)
}

pub(crate) struct AuthRequest {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
}

pub(crate) struct LoopbackAuthRequest {
    pub auth_url: String,
    pub state: String,
    pub code_verifier: String,
    pub redirect_uri: String,
}

pub(crate) fn build_auth_request(
    client_id: &str,
    redirect_uri: &str,
) -> Result<AuthRequest, AuthError> {
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

pub(crate) fn build_loopback_auth_request(client_id: &str) -> Result<LoopbackAuthRequest, AuthError> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|err| format!("Failed to reserve loopback redirect port: {err}"))?;
    let port = listener
        .local_addr()
        .map_err(|err| format!("Failed to read loopback redirect port: {err}"))?
        .port();
    drop(listener);

    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let request = build_auth_request(client_id, &redirect_uri)?;

    Ok(LoopbackAuthRequest {
        auth_url: request.auth_url,
        state: request.state,
        code_verifier: request.code_verifier,
        redirect_uri,
    })
}

pub(crate) async fn wait_for_loopback_callback(
    redirect_uri: &str,
    wait_timeout: Duration,
) -> Result<String, AuthError> {
    let redirect = Url::parse(redirect_uri).map_err(|err| format!("Invalid redirect URI: {err}"))?;
    let host = redirect
        .host_str()
        .ok_or_else(|| "Missing redirect host".to_string())?;
    let port = redirect
        .port_or_known_default()
        .ok_or_else(|| "Missing redirect port".to_string())?;
    let bind_addr = format!("{host}:{port}");

    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|err| format!("Failed to bind loopback listener on {bind_addr}: {err}"))?;
    let (mut socket, _peer) = timeout(wait_timeout, listener.accept())
        .await
        .map_err(|_| "Timed out waiting for Microsoft authorization callback.".to_string())?
        .map_err(|err| format!("Failed to accept Microsoft authorization callback: {err}"))?;

    let mut buffer = vec![0u8; 8192];
    let size = timeout(Duration::from_secs(30), socket.read(&mut buffer))
        .await
        .map_err(|_| "Timed out reading Microsoft authorization callback.".to_string())?
        .map_err(|err| format!("Failed to read Microsoft authorization callback: {err}"))?;
    if size == 0 {
        return Err("Received empty Microsoft authorization callback.".to_string().into());
    }

    let request = String::from_utf8_lossy(&buffer[..size]).to_string();
    let request_line = request
        .lines()
        .next()
        .ok_or_else(|| "Malformed Microsoft authorization callback request.".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    if method != "GET" || target.is_empty() {
        return Err("Unexpected Microsoft authorization callback request."
            .to_string()
            .into());
    }

    let callback_url = format!("http://{bind_addr}{target}");
    let expected_path = redirect.path().to_string();
    let callback = Url::parse(&callback_url)
        .map_err(|err| format!("Invalid Microsoft callback URL: {err}"))?;
    if callback.path() != expected_path {
        return Err("Microsoft callback path did not match expected redirect path."
            .to_string()
            .into());
    }

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n<!doctype html><html><head><meta charset=\"utf-8\" /><title>Atlas Sign-in</title></head><body style=\"font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; padding: 2rem; color: #111; background: #fff;\"><h1 style=\"margin:0 0 0.75rem 0; font-size:1.2rem;\">Sign-in complete</h1><p style=\"margin:0; font-size:1rem;\">You can close this window and return to Atlas Launcher.</p></body></html>";
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;

    Ok(callback_url)
}

pub(crate) fn parse_auth_callback(
    callback_url: &str,
    expected_state: &str,
) -> Result<String, AuthError> {
    let url =
        Url::parse(callback_url).map_err(|err| format!("Invalid auth callback URL: {err}"))?;
    parse_auth_callback_url(&url, expected_state)
}

fn parse_auth_callback_url(url: &Url, expected_state: &str) -> Result<String, AuthError> {
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
        return Err(format!("Microsoft sign-in failed: {error}{description}").into());
    }

    if params.get("state").map(String::as_str) != Some(expected_state) {
        return Err("Sign-in state did not match. Please try again."
            .to_string()
            .into());
    }

    Ok(params
        .get("code")
        .cloned()
        .ok_or_else(|| "Missing authorization code in redirect.".to_string())?)
}

fn parse_pairs(raw: &str, params: &mut HashMap<String, String>) {
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        params.entry(key.into()).or_insert(value.into());
    }
}
