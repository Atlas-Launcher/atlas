use crate::models::AuthSession;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::net::http::HttpClient;
use super::error::AuthError;
use super::minecraft;
use super::ms::DeviceTokenResponse;
use super::xbox;

pub(crate) async fn session_from_ms_token<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    ms_access_token: &str,
    refresh_token: Option<String>,
    fallback_refresh_token: Option<String>,
) -> Result<AuthSession, AuthError> {
    let xbl = xbox::authenticate(http, ms_access_token).await?;
    let xsts = xbox::xsts(http, &xbl.token).await?;
    let uhs = xsts
        .display_claims
        .xui
        .get(0)
        .map(|claim| claim.uhs.clone())
        .ok_or_else(|| "Missing Xbox user hash".to_string())?;
    let mc = minecraft::login(http, &xsts.token, &uhs).await?;

    let profile = minecraft::profile(http, &mc.access_token).await?;
    minecraft::verify_entitlements(http, &mc.access_token).await?;
    let refresh_token = refresh_token
        .or(fallback_refresh_token)
        .ok_or_else(|| "Missing refresh token from Microsoft login.".to_string())?;

    Ok(AuthSession {
        access_token: mc.access_token,
        access_token_expires_at: unix_timestamp().saturating_add(mc.expires_in),
        refresh_token: Some(refresh_token),
        client_id: client_id.to_string(),
        profile,
    })
}

pub(crate) async fn session_from_refresh<H: HttpClient + ?Sized>(
    http: &H,
    client_id: &str,
    refreshed: DeviceTokenResponse,
    fallback_refresh: Option<String>,
) -> Result<AuthSession, AuthError> {
    session_from_ms_token(
        http,
        client_id,
        &refreshed.access_token,
        refreshed.refresh_token,
        fallback_refresh,
    )
    .await
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
