use crate::models::DeviceCodeResponse;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use reqwest::Client;
use serde::Deserialize;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
use url::Url;

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
  pub scope: String
}

#[derive(Debug, Deserialize)]
struct DeviceTokenError {
  error: String,
  #[allow(dead_code)]
  error_description: Option<String>
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

pub async fn login_with_redirect_token<F>(
  client_id: &str,
  open_url: F
) -> Result<DeviceTokenResponse, String>
where
  F: FnOnce(String) -> Result<(), String>
{
  let (auth_url, redirect_uri, code_verifier, state, listener) =
    prepare_redirect_login(client_id).await?;
  open_url(auth_url)?;

  let code = wait_for_auth_code(listener, &state).await?;
  exchange_auth_code(client_id, &code, &redirect_uri, &code_verifier).await
}

pub(crate) async fn poll_device_token(
  client_id: &str,
  device_code: &str
) -> Result<DeviceTokenResponse, String> {
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

pub(crate) async fn refresh_token(
  client_id: &str,
  refresh_token: &str
) -> Result<DeviceTokenResponse, String> {
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

async fn respond_to_browser(
  socket: &mut tokio::net::TcpStream,
  success: bool
) -> Result<(), String> {
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
