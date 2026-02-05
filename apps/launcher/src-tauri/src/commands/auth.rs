use crate::auth;
use crate::config;
use crate::models::{AtlasProfile, DeviceCodeResponse, Profile};
use crate::settings;
use crate::state::AppState;

#[tauri::command]
pub async fn start_device_code() -> Result<DeviceCodeResponse, String> {
    let settings = settings::load_settings().unwrap_or_default();
    let client_id = config::resolve_client_id(&settings);
    auth::start_device_code(&client_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn begin_deeplink_login(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = config::resolve_client_id(&settings);
    let (pending, auth_url) = auth::begin_deeplink_login(&client_id, config::DEFAULT_REDIRECT_URI)
        .map_err(|err| err.to_string())?;
    auth::save_pending_auth(&pending).map_err(|err| err.to_string())?;
    let mut guard = state
        .pending_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(pending);
    Ok(auth_url)
}

#[tauri::command]
pub async fn complete_deeplink_login(
    state: tauri::State<'_, AppState>,
    callback_url: Option<String>,
) -> Result<Profile, String> {
    let pending = {
        let guard = state
            .pending_auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        if let Some(pending) = guard.as_ref() {
            pending.clone()
        } else {
            auth::load_pending_auth()
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "No pending sign-in found. Start sign-in again.".to_string())?
        }
    };

    let callback = callback_url
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| "Missing auth callback URL.".to_string())?;

    let session = auth::complete_deeplink_login(&callback, pending)
        .await
        .map_err(|err| err.to_string())?;
    let profile = session.profile.clone();
    auth::save_session(&session).map_err(|err| err.to_string())?;
    auth::clear_pending_auth().map_err(|err| err.to_string())?;
    let mut pending_guard = state
        .pending_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *pending_guard = None;

    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(profile)
}

#[tauri::command]
pub async fn complete_device_code(
    state: tauri::State<'_, AppState>,
    device_code: String,
) -> Result<Profile, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = config::resolve_client_id(&settings);
    let session = auth::complete_device_code(&client_id, &device_code)
        .await
        .map_err(|err| err.to_string())?;
    let profile = session.profile.clone();
    auth::save_session(&session).map_err(|err| err.to_string())?;
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(profile)
}

#[tauri::command]
pub async fn restore_session(state: tauri::State<'_, AppState>) -> Result<Option<Profile>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = config::resolve_client_id(&settings);
    let session = auth::load_session().map_err(|err| err.to_string())?;
    let Some(mut session) = session else {
        return Ok(None);
    };

    if session.client_id.trim().is_empty() {
        session.client_id = client_id;
    }

    let session = auth::ensure_fresh_session(session)
        .await
        .map_err(|err| err.to_string())?;
    auth::save_session(&session).map_err(|err| err.to_string())?;

    let profile = session.profile.clone();
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(Some(profile))
}

#[tauri::command]
pub fn sign_out(state: tauri::State<'_, AppState>) -> Result<(), String> {
    auth::clear_session().map_err(|err| err.to_string())?;
    auth::clear_pending_auth().map_err(|err| err.to_string())?;
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub async fn begin_atlas_login(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let auth_base_url = config::resolve_atlas_auth_base_url(&settings);
    let client_id = config::resolve_atlas_client_id();
    let redirect_uri = config::resolve_atlas_redirect_uri();
    let (pending, auth_url) = auth::begin_atlas_login(&auth_base_url, &client_id, &redirect_uri)
        .map_err(|err| err.to_string())?;
    auth::save_pending_atlas_auth(&pending).map_err(|err| err.to_string())?;
    let mut guard = state
        .pending_atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(pending);
    Ok(auth_url)
}

#[tauri::command]
pub async fn complete_atlas_login(
    state: tauri::State<'_, AppState>,
    callback_url: Option<String>,
) -> Result<AtlasProfile, String> {
    let pending = {
        let guard = state
            .pending_atlas_auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        if let Some(pending) = guard.as_ref() {
            pending.clone()
        } else {
            auth::load_pending_atlas_auth()
                .map_err(|err| err.to_string())?
                .ok_or_else(|| "No pending Atlas sign-in found. Start sign-in again.".to_string())?
        }
    };

    let callback = callback_url
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| "Missing auth callback URL.".to_string())?;

    let session = auth::complete_atlas_login(&callback, pending)
        .await
        .map_err(|err| err.to_string())?;
    let profile = session.profile.clone();
    auth::save_atlas_session(&session).map_err(|err| err.to_string())?;
    auth::clear_pending_atlas_auth().map_err(|err| err.to_string())?;
    let mut pending_guard = state
        .pending_atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *pending_guard = None;

    let mut guard = state
        .atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(profile)
}

#[tauri::command]
pub async fn restore_atlas_session(
    state: tauri::State<'_, AppState>,
) -> Result<Option<AtlasProfile>, String> {
    let session = auth::load_atlas_session().map_err(|err| err.to_string())?;
    let Some(session) = session else {
        return Ok(None);
    };

    let session = auth::ensure_fresh_atlas_session(session)
        .await
        .map_err(|err| err.to_string())?;
    auth::save_atlas_session(&session).map_err(|err| err.to_string())?;

    let profile = session.profile.clone();
    let mut guard = state
        .atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(Some(profile))
}

#[tauri::command]
pub fn atlas_sign_out(state: tauri::State<'_, AppState>) -> Result<(), String> {
    auth::clear_atlas_session().map_err(|err| err.to_string())?;
    auth::clear_pending_atlas_auth().map_err(|err| err.to_string())?;
    let mut guard = state
        .atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = None;
    let mut pending_guard = state
        .pending_atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *pending_guard = None;
    Ok(())
}
