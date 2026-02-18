use crate::auth;
use crate::config;
use crate::models::{
    AtlasProfile, DeviceCodeResponse, LauncherLinkComplete, LauncherLinkSession, Profile,
};
use crate::settings;
use crate::state::AppState;
use crate::telemetry;
use atlas_client::hub::{HubClient, LauncherLinkCompleteRequest, LauncherMinecraftPayload};
use tauri::Manager;

#[tauri::command]
pub async fn start_device_code() -> Result<DeviceCodeResponse, String> {
    let settings = settings::load_settings().unwrap_or_default();
    let client_id = config::resolve_client_id(&settings);
    auth::start_device_code(&client_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) -> Result<(), String> {
    focus_window_internal(&app, "main")
}

#[tauri::command]
pub fn focus_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    focus_window_internal(&app, label.trim())
}

fn focus_window_internal(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    let target_label = if label.is_empty() { "main" } else { label };
    let window = app
        .get_webview_window(target_label)
        .ok_or_else(|| format!("Window not found: {target_label}"))?;
    if window.is_minimized().map_err(|err| err.to_string())? {
        window.unminimize().map_err(|err| err.to_string())?;
    }
    if !window.is_visible().map_err(|err| err.to_string())? {
        window.show().map_err(|err| err.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
    }
    window.set_focus().map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn begin_deeplink_login(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = config::resolve_client_id(&settings);
    let (pending, auth_url) =
        auth::begin_deeplink_login(&client_id).map_err(|err| err.to_string())?;
    auth::save_pending_auth(&pending).map_err(|err| err.to_string())?;
    let mut guard = state
        .pending_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(pending);
    Ok(auth_url)
}

#[tauri::command]
pub async fn complete_loopback_login(state: tauri::State<'_, AppState>) -> Result<Profile, String> {
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

    let session = auth::complete_loopback_login(pending)
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
pub async fn start_atlas_device_code(
    state: tauri::State<'_, AppState>,
) -> Result<DeviceCodeResponse, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);
    let client_id = config::resolve_atlas_client_id();
    telemetry::info(format!(
        "Launcher requested Atlas device code (hub_url={hub_url}, client_id={client_id})."
    ));
    auth::start_atlas_device_code(&hub_url, &client_id)
        .await
        .map_err(|err| err.to_string())
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
pub async fn complete_atlas_device_code(
    state: tauri::State<'_, AppState>,
    device_code: String,
    interval_seconds: u64,
) -> Result<AtlasProfile, String> {
    let started = std::time::Instant::now();
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);
    let auth_base_url = config::resolve_atlas_auth_base_url(&settings);
    let client_id = config::resolve_atlas_client_id();
    telemetry::info(format!(
        "Launcher waiting for Atlas device approval (hub_url={hub_url}, auth_base_url={auth_base_url}, interval={}s).",
        interval_seconds
    ));
    let session = auth::complete_atlas_device_code(
        &hub_url,
        &auth_base_url,
        &client_id,
        &device_code,
        interval_seconds,
    )
    .await
    .map_err(|err| {
        telemetry::error(format!(
            "Atlas device-code completion failed after {}ms: {}",
            started.elapsed().as_millis(),
            err
        ));
        err.to_string()
    })?;
    let profile = session.profile.clone();
    auth::save_atlas_session(&session).map_err(|err| err.to_string())?;
    let mut guard = state
        .atlas_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    telemetry::info(format!(
        "Atlas device-code completion succeeded for user {} (elapsed={}ms).",
        profile.id,
        started.elapsed().as_millis()
    ));
    Ok(profile)
}

#[tauri::command]
pub async fn restore_atlas_session(
    state: tauri::State<'_, AppState>,
) -> Result<Option<AtlasProfile>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let session = auth::load_atlas_session().map_err(|err| err.to_string())?;
    let Some(session) = session else {
        return Ok(None);
    };

    let mut session = auth::ensure_fresh_atlas_session(session)
        .await
        .map_err(|err| err.to_string())?;
    session.profile.mojang_uuid = session
        .profile
        .mojang_uuid
        .as_deref()
        .and_then(canonicalize_mojang_uuid);
    if session
        .profile
        .mojang_uuid
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        session = auth::refresh_atlas_profile(session)
            .await
            .map_err(|err| err.to_string())?;
    }

    if session
        .profile
        .mojang_uuid
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        let hub_url = config::resolve_atlas_hub_url(&settings);
        if let Ok(hub) = HubClient::new(&hub_url) {
            match hub.get_mojang_info(&session.access_token).await {
                Ok(info) => {
                    if let Some(uuid) = info.uuid {
                        session.profile.mojang_uuid = canonicalize_mojang_uuid(&uuid);
                    }
                    if let Some(username) = info.username {
                        session.profile.mojang_username = Some(username);
                    }
                }
                Err(err) => {
                    telemetry::warn(format!(
                        "Failed to refresh Mojang info during restore: {err}"
                    ));
                }
            }
        }
    }
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

#[tauri::command]
pub async fn create_launcher_link_session(
    state: tauri::State<'_, AppState>,
) -> Result<LauncherLinkSession, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);
    let hub = HubClient::new(&hub_url).map_err(|err| err.to_string())?;
    hub.create_launcher_link_session()
        .await
        .map(LauncherLinkSession::from)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn complete_launcher_link_session(
    state: tauri::State<'_, AppState>,
    link_session_id: String,
    proof: String,
    minecraft_uuid: String,
    minecraft_name: String,
) -> Result<LauncherLinkComplete, String> {
    telemetry::info(format!(
        "Completing launcher link session {link_session_id} for {minecraft_name} ({minecraft_uuid})."
    ));
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);
    let hub = HubClient::new(&hub_url).map_err(|err| err.to_string())?;
    let payload = LauncherLinkCompleteRequest {
        link_session_id,
        proof,
        minecraft: LauncherMinecraftPayload {
            uuid: minecraft_uuid,
            name: minecraft_name,
        },
    };

    let mut result = hub
        .complete_launcher_link_session(&payload)
        .await
        .map(LauncherLinkComplete::from)
        .map_err(|err| err.to_string())?;

    let mut warning: Option<String> = None;
    let session = match auth::load_atlas_session() {
        Ok(Some(session)) => Some(session),
        Ok(None) => {
            warning = Some("Atlas session missing after link completion.".to_string());
            None
        }
        Err(err) => {
            warning = Some(format!("Atlas session load failed: {err}"));
            None
        }
    };

    if let Some(session) = session {
        let session = match auth::ensure_fresh_atlas_session(session).await {
            Ok(session) => Some(session),
            Err(err) => {
                warning = Some(format!("Atlas session refresh failed: {err}"));
                None
            }
        };

        if let Some(session) = session {
            let mut session = match auth::refresh_atlas_profile(session).await {
                Ok(session) => Some(session),
                Err(err) => {
                    warning = Some(format!("Atlas profile refresh failed: {err}"));
                    None
                }
            };

            if let Some(ref mut session) = session {
                match hub.get_mojang_info(&session.access_token).await {
                    Ok(info) => {
                        if let Some(uuid) = info.uuid.clone() {
                            session.profile.mojang_uuid = canonicalize_mojang_uuid(&uuid);
                        }
                        if let Some(username) = info.username.clone() {
                            session.profile.mojang_username = Some(username);
                        }
                    }
                    Err(err) => {
                        telemetry::warn(format!(
                            "Failed to fetch Mojang info after link completion: {err}"
                        ));
                        warning = Some(format!("Mojang info refresh failed: {err}"));
                    }
                }

                let _ = auth::save_atlas_session(session);
                if let Ok(mut guard) = state.atlas_auth.lock() {
                    *guard = Some(session.clone());
                }
            }
        }
    }

    if let Some(message) = warning.as_ref() {
        telemetry::warn(message);
    }
    result.warning = warning;

    Ok(result)
}

fn canonicalize_mojang_uuid(value: &str) -> Option<String> {
    let lower = value.trim().to_ascii_lowercase();
    let candidate = lower.strip_prefix("urn:uuid:").unwrap_or(&lower);
    let hex = candidate
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>();
    if hex.len() == 32 {
        Some(hex)
    } else {
        None
    }
}
