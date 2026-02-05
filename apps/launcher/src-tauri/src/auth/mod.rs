mod error;
mod flow;
mod minecraft;
mod ms;
mod pending;
mod session;
mod xbox;

use crate::models::{AuthSession, DeviceCodeResponse};
use crate::net::http::ReqwestHttpClient;

pub use error::AuthError;
pub use pending::{clear_pending_auth, load_pending_auth, save_pending_auth, PendingAuth};
pub use session::{clear_session, ensure_fresh_session, load_session, save_session};

pub async fn start_device_code(client_id: &str) -> Result<DeviceCodeResponse, AuthError> {
    let http = ReqwestHttpClient::new();
    ms::start_device_code(&http, client_id).await
}

pub async fn complete_device_code(
    client_id: &str,
    device_code: &str,
) -> Result<AuthSession, AuthError> {
    let http = ReqwestHttpClient::new();
    let token = ms::poll_device_token(&http, client_id, device_code).await?;
    let refresh_token = token.refresh_token.clone();
    flow::session_from_ms_token(&http, client_id, &token.access_token, refresh_token, None).await
}

pub fn begin_deeplink_login(
    client_id: &str,
    redirect_uri: &str,
) -> Result<(PendingAuth, String), AuthError> {
    let request = ms::build_auth_request(client_id, redirect_uri)?;
    let pending = PendingAuth {
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.to_string(),
        state: request.state,
        code_verifier: request.code_verifier,
    };
    Ok((pending, request.auth_url))
}

pub async fn complete_deeplink_login(
    callback_url: &str,
    pending: PendingAuth,
) -> Result<AuthSession, AuthError> {
    let http = ReqwestHttpClient::new();
    let code = ms::parse_auth_callback(callback_url, &pending.state)?;
    let token = ms::exchange_auth_code(
        &http,
        &pending.client_id,
        &code,
        &pending.redirect_uri,
        &pending.code_verifier,
    )
    .await?;
    let refresh_token = token.refresh_token.clone();
    flow::session_from_ms_token(
        &http,
        &pending.client_id,
        &token.access_token,
        refresh_token,
        None,
    )
    .await
}

#[cfg(test)]
mod tests;
