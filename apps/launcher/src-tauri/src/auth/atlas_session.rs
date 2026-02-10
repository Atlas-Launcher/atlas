use crate::models::{AtlasProfile, AtlasSession};
use crate::paths::{atlas_auth_store_path, ensure_dir, file_exists};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use super::atlas;
use super::error::AuthError;

pub fn load_atlas_session() -> Result<Option<AtlasSession>, AuthError> {
    let path = atlas_auth_store_path()?;
    if !file_exists(&path) {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read Atlas session: {err}"))?;
    let session = serde_json::from_slice::<AtlasSession>(&bytes)
        .map_err(|err| format!("Failed to parse Atlas session: {err}"))?;
    Ok(Some(session))
}

pub fn save_atlas_session(session: &AtlasSession) -> Result<(), AuthError> {
    let path = atlas_auth_store_path()?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_vec_pretty(session)
        .map_err(|err| format!("Failed to serialize Atlas session: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Failed to write Atlas session: {err}"))?;
    Ok(())
}

pub fn clear_atlas_session() -> Result<(), AuthError> {
    let path = atlas_auth_store_path()?;
    if file_exists(&path) {
        fs::remove_file(&path).map_err(|err| format!("Failed to remove Atlas session: {err}"))?;
    }
    Ok(())
}

pub async fn ensure_fresh_atlas_session(session: AtlasSession) -> Result<AtlasSession, AuthError> {
    if !needs_refresh(&session) {
        return Ok(session);
    }
    refresh_atlas_session(&session).await
}

pub async fn refresh_atlas_profile(session: AtlasSession) -> Result<AtlasSession, AuthError> {
    let user_info = atlas::fetch_user_info(&session.auth_base_url, &session.access_token).await?;

    Ok(AtlasSession {
        profile: AtlasProfile {
            id: user_info.sub,
            email: user_info.email,
            name: user_info.name,
            mojang_username: user_info.mojang_username,
            mojang_uuid: normalize_mojang_uuid(user_info.mojang_uuid),
        },
        ..session
    })
}

fn needs_refresh(session: &AtlasSession) -> bool {
    let now = unix_timestamp();
    if session.access_token_expires_at == 0 {
        return true;
    }
    now + 300 >= session.access_token_expires_at
}

async fn refresh_atlas_session(session: &AtlasSession) -> Result<AtlasSession, AuthError> {
    let refresh_token = session
        .refresh_token
        .clone()
        .ok_or_else(|| "Missing Atlas refresh token; please sign in again.".to_string())?;

    let refreshed = atlas::refresh_token(
        &session.auth_base_url,
        &session.client_id,
        &refresh_token,
    )
    .await?;
    let user_info =
        atlas::fetch_user_info(&session.auth_base_url, &refreshed.access_token).await?;

    Ok(AtlasSession {
        access_token: refreshed.access_token,
        access_token_expires_at: unix_timestamp().saturating_add(refreshed.expires_in),
        refresh_token: refreshed.refresh_token.or(Some(refresh_token)),
        client_id: session.client_id.clone(),
        auth_base_url: session.auth_base_url.clone(),
        profile: AtlasProfile {
            id: user_info.sub,
            email: user_info.email,
            name: user_info.name,
            mojang_username: user_info.mojang_username,
            mojang_uuid: normalize_mojang_uuid(user_info.mojang_uuid),
        },
    })
}

fn normalize_mojang_uuid(value: Option<String>) -> Option<String> {
    value.map(|raw| raw.trim().to_ascii_lowercase().replace('-', ""))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
