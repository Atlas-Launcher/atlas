use crate::models::{AuthSession, DeviceCodeResponse, Profile};
use crate::paths::{auth_store_path, ensure_dir, file_exists};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use url::Url;

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const AUTHORIZE_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS_URL: &str = "https://api.minecraftservices.com/entitlements/mcstore";
const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Deserialize)]
struct DeviceTokenResponse {
  access_token: String,
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
  let refresh_token = token.refresh_token.clone();
  session_from_ms_token(client_id, &token.access_token, refresh_token, None).await
}

pub async fn login_with_redirect<F>(client_id: &str, open_url: F) -> Result<AuthSession, String>
where
  F: FnOnce(String) -> Result<(), String>
{
  let (auth_url, redirect_uri, code_verifier, state, listener) =
    prepare_redirect_login(client_id).await?;
  open_url(auth_url)?;

  let code = wait_for_auth_code(listener, &state).await?;
  let token = exchange_auth_code(client_id, &code, &redirect_uri, &code_verifier).await?;
  let refresh_token = token.refresh_token.clone();
  session_from_ms_token(client_id, &token.access_token, refresh_token, None).await
}

pub fn load_session() -> Result<Option<AuthSession>, String> {
  let path = auth_store_path()?;
  if !file_exists(&path) {
    return Ok(None);
  }
  let bytes = fs::read(&path).map_err(|err| format!("Failed to read auth session: {err}"))?;
  let session = serde_json::from_slice::<AuthSession>(&bytes)
    .map_err(|err| format!("Failed to parse auth session: {err}"))?;
  Ok(Some(session))
}

pub fn save_session(session: &AuthSession) -> Result<(), String> {
  let path = auth_store_path()?;
  if let Some(parent) = path.parent() {
    ensure_dir(parent)?;
  }
  let payload =
    serde_json::to_vec_pretty(session).map_err(|err| format!("Failed to serialize auth: {err}"))?;
  fs::write(&path, payload).map_err(|err| format!("Failed to write auth session: {err}"))?;
  Ok(())
}

pub fn clear_session() -> Result<(), String> {
  let path = auth_store_path()?;
  if file_exists(&path) {
    fs::remove_file(&path).map_err(|err| format!("Failed to remove auth session: {err}"))?;
  }
  Ok(())
}

pub async fn ensure_fresh_session(session: AuthSession) -> Result<AuthSession, String> {
  if !needs_refresh(&session) {
    return Ok(session);
  }
  refresh_session(&session).await
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

async fn prepare_redirect_login(
  client_id: &str
) -> Result<(String, String, String, String, TcpListener), String> {
  let listener = TcpListener::bind("127.0.0.1:0")
    .await
    .map_err(|err| format!("Failed to bind redirect listener: {err}"))?;
  let addr = listener
    .local_addr()
    .map_err(|err| format!("Failed to read redirect address: {err}"))?;

  let redirect_uri = format!("http://127.0.0.1:{}/callback", addr.port());
  let state = random_url_safe(16);
  let code_verifier = random_url_safe(64);
  let code_challenge = code_challenge_s256(&code_verifier);

  let mut url = Url::parse(AUTHORIZE_URL).map_err(|err| format!("Invalid authorize URL: {err}"))?;
  url
    .query_pairs_mut()
    .append_pair("client_id", client_id)
    .append_pair("response_type", "code")
    .append_pair("redirect_uri", &redirect_uri)
    .append_pair("response_mode", "query")
    .append_pair("scope", "XboxLive.signin offline_access")
    .append_pair("code_challenge", &code_challenge)
    .append_pair("code_challenge_method", "S256")
    .append_pair("state", &state)
    .append_pair("prompt", "select_account");

  Ok((url.to_string(), redirect_uri, code_verifier, state, listener))
}

fn random_url_safe(len: usize) -> String {
  let mut bytes = vec![0u8; len];
  OsRng.fill_bytes(&mut bytes);
  URL_SAFE_NO_PAD.encode(&bytes)
}

fn code_challenge_s256(verifier: &str) -> String {
  use sha2::{Digest, Sha256};
  let mut hasher = Sha256::new();
  hasher.update(verifier.as_bytes());
  let digest = hasher.finalize();
  URL_SAFE_NO_PAD.encode(digest)
}

async fn wait_for_auth_code(listener: TcpListener, expected_state: &str) -> Result<String, String> {
  let timeout = Duration::from_secs(300);
  let accept = tokio::time::timeout(timeout, listener.accept()).await;
  let (mut socket, addr) = accept
    .map_err(|_| "Sign-in timed out. Please try again.".to_string())?
    .map_err(|err| format!("Failed to accept redirect: {err}"))?;

  let request = read_http_request(&mut socket, addr).await?;
  let path = request
    .split_whitespace()
    .nth(1)
    .ok_or_else(|| "Invalid redirect request.".to_string())?;
  let url = Url::parse(&format!("http://localhost{}", path))
    .map_err(|err| format!("Invalid redirect URL: {err}"))?;

  let mut code: Option<String> = None;
  let mut state: Option<String> = None;
  let mut error: Option<String> = None;

  for (key, value) in url.query_pairs() {
    match key.as_ref() {
      "code" => code = Some(value.to_string()),
      "state" => state = Some(value.to_string()),
      "error" => error = Some(value.to_string()),
      _ => {}
    }
  }

  respond_to_browser(&mut socket, error.is_none()).await?;

  if let Some(error) = error {
    return Err(format!("Microsoft sign-in failed: {error}"));
  }

  if state.as_deref() != Some(expected_state) {
    return Err("Sign-in state did not match. Please try again.".to_string());
  }

  code.ok_or_else(|| "Missing authorization code in redirect.".to_string())
}

async fn read_http_request(
  socket: &mut tokio::net::TcpStream,
  _addr: SocketAddr
) -> Result<String, String> {
  let mut buffer = vec![0u8; 4096];
  let read = socket
    .read(&mut buffer)
    .await
    .map_err(|err| format!("Failed to read redirect request: {err}"))?;
  if read == 0 {
    return Err("Empty redirect request.".to_string());
  }
  let request = String::from_utf8_lossy(&buffer[..read]).to_string();
  Ok(request.lines().next().unwrap_or_default().to_string())
}

async fn respond_to_browser(socket: &mut tokio::net::TcpStream, success: bool) -> Result<(), String> {
  let body = if success {
    "Sign-in complete. You can return to the launcher."
  } else {
    "Sign-in failed. You can return to the launcher and try again."
  };
  let response = format!(
    "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\r\n{}",
    body.len(),
    body
  );
  socket
    .write_all(response.as_bytes())
    .await
    .map_err(|err| format!("Failed to respond to browser: {err}"))?;
  Ok(())
}

async fn exchange_auth_code(
  client_id: &str,
  code: &str,
  redirect_uri: &str,
  code_verifier: &str
) -> Result<DeviceTokenResponse, String> {
  let client = Client::new();
  let params = [
    ("client_id", client_id),
    ("grant_type", "authorization_code"),
    ("code", code),
    ("redirect_uri", redirect_uri),
    ("code_verifier", code_verifier)
  ];

  let response = client
    .post(TOKEN_URL)
    .form(&params)
    .send()
    .await
    .map_err(|err| format!("Authorization code exchange failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Authorization code exchange failed ({status}): {text}"));
  }

  response
    .json::<DeviceTokenResponse>()
    .await
    .map_err(|err| format!("Failed to parse authorization response: {err}"))
}

fn unix_timestamp() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_secs()
}

fn needs_refresh(session: &AuthSession) -> bool {
  let now = unix_timestamp();
  if session.access_token_expires_at == 0 {
    return true;
  }
  now + 300 >= session.access_token_expires_at
}

async fn refresh_session(session: &AuthSession) -> Result<AuthSession, String> {
  let refresh_token = session
    .refresh_token
    .clone()
    .ok_or_else(|| "Missing refresh token; please sign in again.".to_string())?;
  let refreshed = refresh_ms_token(&session.client_id, &refresh_token).await?;
  let fallback_refresh = refreshed.refresh_token.clone().or(Some(refresh_token));
  session_from_ms_token(
    &session.client_id,
    &refreshed.access_token,
    refreshed.refresh_token,
    fallback_refresh
  )
  .await
}

async fn refresh_ms_token(client_id: &str, refresh_token: &str) -> Result<DeviceTokenResponse, String> {
  let client = Client::new();
  let params = [
    ("client_id", client_id),
    ("grant_type", "refresh_token"),
    ("refresh_token", refresh_token)
  ];

  let response = client
    .post(TOKEN_URL)
    .form(&params)
    .send()
    .await
    .map_err(|err| format!("Refresh token request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Refresh token request failed ({status}): {text}"));
  }

  response
    .json::<DeviceTokenResponse>()
    .await
    .map_err(|err| format!("Failed to parse refresh response: {err}"))
}

async fn session_from_ms_token(
  client_id: &str,
  ms_access_token: &str,
  refresh_token: Option<String>,
  fallback_refresh_token: Option<String>
) -> Result<AuthSession, String> {
  let xbl = xbox_authenticate(ms_access_token).await?;
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
  let refresh_token = refresh_token
    .or(fallback_refresh_token)
    .ok_or_else(|| "Missing refresh token from Microsoft login.".to_string())?;

  Ok(AuthSession {
    access_token: mc.access_token,
    access_token_expires_at: unix_timestamp().saturating_add(mc.expires_in),
    refresh_token: Some(refresh_token),
    client_id: client_id.to_string(),
    profile
  })
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
