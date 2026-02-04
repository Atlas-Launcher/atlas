mod auth;
mod launcher;
mod models;
mod paths;
mod settings;
mod state;

use crate::models::{AppSettings, DeviceCodeResponse, LaunchOptions, Profile};
use crate::state::AppState;

const DEFAULT_MS_CLIENT_ID: &str = "e253b0f5-35af-488a-abf2-54149dbd094d";
const DEFAULT_REDIRECT_URI: &str = "atlas://auth";

fn resolve_client_id(settings: &AppSettings) -> String {
    settings
        .ms_client_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_MS_CLIENT_ID.to_string())
}

#[tauri::command]
fn get_default_game_dir() -> String {
    paths::default_game_dir().to_string_lossy().to_string()
}

#[tauri::command]
async fn start_device_code() -> Result<DeviceCodeResponse, String> {
    let settings = settings::load_settings().unwrap_or_default();
    let client_id = resolve_client_id(&settings);
    auth::start_device_code(&client_id).await
}

#[tauri::command]
async fn begin_deeplink_login(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = resolve_client_id(&settings);
    let (pending, auth_url) = auth::begin_deeplink_login(&client_id, DEFAULT_REDIRECT_URI)?;
    auth::save_pending_auth(&pending)?;
    let mut guard = state
        .pending_auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(pending);
    Ok(auth_url)
}

#[tauri::command]
async fn complete_deeplink_login(
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
            auth::load_pending_auth()?
                .ok_or_else(|| "No pending sign-in found. Start sign-in again.".to_string())?
        }
    };

    let callback = callback_url
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| "Missing auth callback URL.".to_string())?;

    let session = auth::complete_deeplink_login(&callback, pending).await?;
    let profile = session.profile.clone();
    auth::save_session(&session)?;
    auth::clear_pending_auth()?;
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
async fn complete_device_code(
    state: tauri::State<'_, AppState>,
    device_code: String,
) -> Result<Profile, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = resolve_client_id(&settings);
    let session = auth::complete_device_code(&client_id, &device_code).await?;
    let profile = session.profile.clone();
    auth::save_session(&session)?;
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(profile)
}

#[tauri::command]
async fn launch_minecraft(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    options: LaunchOptions,
) -> Result<(), String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = resolve_client_id(&settings);
    let mut session = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?
        .clone();

    if session.is_none() {
        session = auth::load_session()?;
    }

    let mut session = session.ok_or_else(|| "Not signed in. Sign in first.".to_string())?;

    if session.client_id.trim().is_empty() {
        session.client_id = client_id;
    }

    let session = auth::ensure_fresh_session(session).await?;
    auth::save_session(&session)?;

    {
        let mut guard = state
            .auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        *guard = Some(session.clone());
    }

    launcher::launch_minecraft(&window, &options, &session).await
}

#[tauri::command]
async fn download_minecraft_files(
    window: tauri::Window,
    options: LaunchOptions,
) -> Result<(), String> {
    launcher::download_minecraft_files(&window, &options).await
}

#[tauri::command]
async fn restore_session(state: tauri::State<'_, AppState>) -> Result<Option<Profile>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = resolve_client_id(&settings);
    let session = auth::load_session()?;
    let Some(mut session) = session else {
        return Ok(None);
    };

    if session.client_id.trim().is_empty() {
        session.client_id = client_id;
    }

    let session = auth::ensure_fresh_session(session).await?;
    auth::save_session(&session)?;

    let profile = session.profile.clone();
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session);
    Ok(Some(profile))
}

#[tauri::command]
fn sign_out(state: tauri::State<'_, AppState>) -> Result<(), String> {
    auth::clear_session()?;
    auth::clear_pending_auth()?;
    let mut guard = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    let guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
fn update_settings(state: tauri::State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
    settings::save_settings(&settings)?;
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    *guard = settings;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState::default())
        .setup(|app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                app.deep_link().register_all()?;
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_default_game_dir,
            start_device_code,
            begin_deeplink_login,
            complete_deeplink_login,
            complete_device_code,
            launch_minecraft,
            download_minecraft_files,
            restore_session,
            sign_out,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
