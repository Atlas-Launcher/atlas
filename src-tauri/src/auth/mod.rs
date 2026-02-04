mod flow;
mod http;
mod minecraft;
mod ms;
mod session;
mod xbox;

use crate::models::{AuthSession, DeviceCodeResponse};

pub use session::{clear_session, ensure_fresh_session, load_session, save_session};

pub async fn start_device_code(client_id: &str) -> Result<DeviceCodeResponse, String> {
  ms::start_device_code(client_id).await
}

pub async fn complete_device_code(client_id: &str, device_code: &str) -> Result<AuthSession, String> {
  let token = ms::poll_device_token(client_id, device_code).await?;
  let refresh_token = token.refresh_token.clone();
  flow::session_from_ms_token(client_id, &token.access_token, refresh_token, None).await
}

pub async fn login_with_redirect<F>(client_id: &str, open_url: F) -> Result<AuthSession, String>
where
  F: FnOnce(String) -> Result<(), String>
{
  let token = ms::login_with_redirect_token(client_id, open_url).await?;
  let refresh_token = token.refresh_token.clone();
  flow::session_from_ms_token(client_id, &token.access_token, refresh_token, None).await
}
