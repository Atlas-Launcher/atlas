mod atlas;
mod atlas_pending;
mod atlas_session;
mod error;
mod flow;
mod minecraft;
mod ms;
mod pending;
mod session;
mod xbox;

use crate::models::{AtlasProfile, AtlasSession, AuthSession, DeviceCodeResponse};
use crate::net::http::ReqwestHttpClient;
use crate::telemetry;
use atlas_client::hub::HubClient;

pub use atlas_pending::{
    clear_pending_atlas_auth, load_pending_atlas_auth, save_pending_atlas_auth, AtlasPendingAuth,
};
pub use atlas_session::{
    clear_atlas_session, ensure_fresh_atlas_session, load_atlas_session, refresh_atlas_profile,
    save_atlas_session,
};
pub use error::AuthError;
pub use pending::{clear_pending_auth, load_pending_auth, save_pending_auth, PendingAuth};
pub use session::{clear_session, ensure_fresh_session, load_session, save_session};

pub async fn start_device_code(client_id: &str) -> Result<DeviceCodeResponse, AuthError> {
    let http = ReqwestHttpClient::new();
    ms::start_device_code(&http, client_id).await
}

pub async fn start_atlas_device_code(
    hub_url: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse, AuthError> {
    atlas::start_device_code(hub_url, client_id)
        .await
        .map(DeviceCodeResponse::from)
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

pub async fn complete_atlas_device_code(
    hub_url: &str,
    auth_base_url: &str,
    client_id: &str,
    device_code: &str,
    interval_seconds: u64,
) -> Result<AtlasSession, AuthError> {
    let started = std::time::Instant::now();
    let token = atlas::poll_device_token(hub_url, client_id, device_code, interval_seconds).await?;
    telemetry::info(format!(
        "Atlas device code token acquired; fetching user info (elapsed={}ms).",
        started.elapsed().as_millis()
    ));
    let profile = match atlas::fetch_user_info(auth_base_url, &token.access_token).await {
        Ok(profile) => {
            telemetry::info(format!(
                "Atlas user info fetch succeeded (elapsed={}ms).",
                started.elapsed().as_millis()
            ));
            AtlasProfile {
                id: profile.sub,
                email: profile.email,
                name: profile.name,
                mojang_username: profile.mojang_username,
                mojang_uuid: profile.mojang_uuid,
            }
        }
        Err(err) => {
            telemetry::warn(format!(
                "Atlas user info fetch failed after device auth; using fallback profile: {}",
                err
            ));
            let mut fallback = AtlasProfile {
                id: derive_device_profile_id(&token.access_token),
                email: None,
                name: Some("Atlas account".to_string()),
                mojang_username: None,
                mojang_uuid: None,
            };

            if let Ok(hub) = HubClient::new(hub_url) {
                if let Ok(info) = hub.get_mojang_info(&token.access_token).await {
                    fallback.mojang_uuid = info.uuid;
                    fallback.mojang_username = info.username;
                    telemetry::info("Atlas fallback profile enriched with Mojang info.");
                }
            }

            fallback
        }
    };

    Ok(AtlasSession {
        access_token: token.access_token,
        access_token_expires_at: unix_timestamp().saturating_add(token.expires_in),
        refresh_token: token.refresh_token,
        client_id: client_id.to_string(),
        auth_base_url: auth_base_url.to_string(),
        profile,
    })
}

pub fn begin_deeplink_login(
    client_id: &str,
) -> Result<(PendingAuth, String), AuthError> {
    let request = ms::build_loopback_auth_request(client_id)?;
    let pending = PendingAuth {
        client_id: client_id.to_string(),
        redirect_uri: request.redirect_uri,
        state: request.state,
        code_verifier: request.code_verifier,
    };
    Ok((pending, request.auth_url))
}

pub async fn complete_loopback_login(pending: PendingAuth) -> Result<AuthSession, AuthError> {
    let callback_url = ms::wait_for_loopback_callback(&pending.redirect_uri, std::time::Duration::from_secs(300)).await?;
    complete_deeplink_login(&callback_url, pending).await
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

pub fn begin_atlas_login(
    auth_base_url: &str,
    client_id: &str,
    redirect_uri: &str,
) -> Result<(AtlasPendingAuth, String), AuthError> {
    let request = atlas::build_auth_request(auth_base_url, client_id, redirect_uri)?;
    let pending = AtlasPendingAuth {
        auth_base_url: auth_base_url.to_string(),
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.to_string(),
        state: request.state,
        code_verifier: request.code_verifier,
    };
    Ok((pending, request.auth_url))
}

pub async fn complete_atlas_login(
    callback_url: &str,
    pending: AtlasPendingAuth,
) -> Result<AtlasSession, AuthError> {
    let code = atlas::parse_auth_callback(callback_url, &pending.state)?;
    let token = atlas::exchange_auth_code(
        &pending.auth_base_url,
        &pending.client_id,
        &code,
        &pending.redirect_uri,
        &pending.code_verifier,
    )
    .await?;
    let profile = atlas::fetch_user_info(&pending.auth_base_url, &token.access_token).await?;

    Ok(AtlasSession {
        access_token: token.access_token,
        access_token_expires_at: unix_timestamp().saturating_add(token.expires_in),
        refresh_token: token.refresh_token,
        client_id: pending.client_id,
        auth_base_url: pending.auth_base_url,
        profile: AtlasProfile {
            id: profile.sub,
            email: profile.email,
            name: profile.name,
            mojang_username: profile.mojang_username,
            mojang_uuid: profile.mojang_uuid,
        },
    })
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn derive_device_profile_id(access_token: &str) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(access_token.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    format!("atlas-device-{}", &hex[..16])
}

#[cfg(test)]
mod tests;
