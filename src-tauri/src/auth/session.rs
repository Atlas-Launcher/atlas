use crate::models::AuthSession;
use crate::paths::{auth_store_path, ensure_dir, file_exists};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::flow;
use super::ms;
use super::error::AuthError;

pub fn load_session() -> Result<Option<AuthSession>, AuthError> {
    let path = auth_store_path()?;
    if !file_exists(&path) {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read auth session: {err}"))?;
    let session = serde_json::from_slice::<AuthSession>(&bytes)
        .map_err(|err| format!("Failed to parse auth session: {err}"))?;
    Ok(Some(session))
}

pub fn save_session(session: &AuthSession) -> Result<(), AuthError> {
    let path = auth_store_path()?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_vec_pretty(session)
        .map_err(|err| format!("Failed to serialize auth: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Failed to write auth session: {err}"))?;
    Ok(())
}

pub fn clear_session() -> Result<(), AuthError> {
    let path = auth_store_path()?;
    if file_exists(&path) {
        fs::remove_file(&path).map_err(|err| format!("Failed to remove auth session: {err}"))?;
    }
    Ok(())
}

pub async fn ensure_fresh_session(session: AuthSession) -> Result<AuthSession, AuthError> {
    if !needs_refresh(&session) {
        return Ok(session);
    }
    refresh_session(&session).await
}

fn needs_refresh(session: &AuthSession) -> bool {
    let now = unix_timestamp();
    if session.access_token_expires_at == 0 {
        return true;
    }
    now + 300 >= session.access_token_expires_at
}

async fn refresh_session(session: &AuthSession) -> Result<AuthSession, AuthError> {
    let http = crate::net::http::ReqwestHttpClient::new();
    let refresh_token = session
        .refresh_token
        .clone()
        .ok_or_else(|| "Missing refresh token; please sign in again.".to_string())?;
    let refreshed = ms::refresh_token(&http, &session.client_id, &refresh_token).await?;
    let fallback_refresh = refreshed.refresh_token.clone().or(Some(refresh_token));
    flow::session_from_refresh(&http, &session.client_id, refreshed, fallback_refresh).await
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
