mod auth;
mod launcher;
mod models;
mod paths;
mod settings;
mod state;

use crate::models::{
    AppSettings, DeviceCodeResponse, FabricLoaderVersion, LaunchOptions, ModEntry, Profile,
    VersionManifestSummary, VersionSummary,
};
use crate::state::AppState;
use reqwest::Client;
use serde::Deserialize;
use std::fs;
use std::path::Component;

const DEFAULT_MS_CLIENT_ID: &str = "e253b0f5-35af-488a-abf2-54149dbd094d";
const DEFAULT_REDIRECT_URI: &str = "atlas://auth";

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

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
async fn get_version_manifest_summary() -> Result<VersionManifestSummary, String> {
    let client = Client::new();
    let manifest: launcher::manifest::VersionManifest =
        launcher::download::fetch_json(&client, launcher::manifest::VERSION_MANIFEST_URL).await?;
    let versions = manifest
        .versions
        .into_iter()
        .map(|version| VersionSummary {
            id: version.id,
            kind: version.kind,
        })
        .collect();
    Ok(VersionManifestSummary {
        latest_release: manifest.latest.release,
        versions,
    })
}

#[tauri::command]
async fn get_fabric_loader_versions(
    minecraft_version: String,
) -> Result<Vec<FabricLoaderVersion>, String> {
    let client = Client::new();
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{minecraft_version}");
    let entries: Vec<FabricLoaderEntry> =
        launcher::download::fetch_json(&client, &url).await?;
    Ok(entries
        .into_iter()
        .map(|entry| FabricLoaderVersion {
            version: entry.loader.version,
            stable: entry.loader.stable,
        })
        .collect())
}

#[tauri::command]
fn list_installed_versions(game_dir: String) -> Result<Vec<String>, String> {
    let base_dir = paths::normalize_path(&game_dir);
    let versions_dir = base_dir.join("versions");
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    let entries = fs::read_dir(&versions_dir)
        .map_err(|err| format!("Failed to read versions dir: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read versions dir entry: {err}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let json_path = path.join(format!("{name}.json"));
        if json_path.exists() {
            versions.push(name);
        }
    }
    versions.sort();
    Ok(versions)
}

#[tauri::command]
fn list_mods(game_dir: String) -> Result<Vec<ModEntry>, String> {
    let base_dir = paths::normalize_path(&game_dir);
    let mods_dir = base_dir.join("mods");
    paths::ensure_dir(&mods_dir)?;

    let mut mods = Vec::new();
    let entries = fs::read_dir(&mods_dir)
        .map_err(|err| format!("Failed to read mods dir: {err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("Failed to read mods dir entry: {err}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_mod_filename(&name) {
            continue;
        }
        let enabled = !name.ends_with(".disabled");
        let display_name = format_mod_display_name(&name);
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("Failed to read mod metadata: {err}"))?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        mods.push(ModEntry {
            file_name: name,
            display_name,
            enabled,
            size: metadata.len(),
            modified,
        });
    }
    mods.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
fn set_mod_enabled(game_dir: String, file_name: String, enabled: bool) -> Result<(), String> {
    let base_dir = paths::normalize_path(&game_dir);
    let mods_dir = base_dir.join("mods");
    paths::ensure_dir(&mods_dir)?;

    let safe_name = sanitize_mod_filename(&file_name)?;
    let current_path = mods_dir.join(&safe_name);
    if !current_path.exists() {
        return Err(format!("Mod {safe_name} not found."));
    }

    let currently_disabled = safe_name.ends_with(".disabled");
    let target_name = match (enabled, currently_disabled) {
        (true, true) => safe_name.trim_end_matches(".disabled").to_string(),
        (false, false) => format!("{safe_name}.disabled"),
        _ => safe_name.clone(),
    };

    if target_name == safe_name {
        return Ok(());
    }

    let target_path = mods_dir.join(&target_name);
    if target_path.exists() {
        return Err(format!(
            "Cannot toggle mod. Target file already exists: {target_name}"
        ));
    }

    fs::rename(&current_path, &target_path)
        .map_err(|err| format!("Failed to rename mod: {err}"))?;
    Ok(())
}

#[tauri::command]
fn delete_mod(game_dir: String, file_name: String) -> Result<(), String> {
    let base_dir = paths::normalize_path(&game_dir);
    let mods_dir = base_dir.join("mods");
    paths::ensure_dir(&mods_dir)?;

    let safe_name = sanitize_mod_filename(&file_name)?;
    let path = mods_dir.join(&safe_name);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| format!("Failed to delete mod: {err}"))?;
    Ok(())
}

fn is_mod_filename(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jar")
        || lower.ends_with(".zip")
        || lower.ends_with(".jar.disabled")
        || lower.ends_with(".zip.disabled")
}

fn format_mod_display_name(name: &str) -> String {
    let trimmed = name.trim_end_matches(".disabled");
    let trimmed = trimmed.trim_end_matches(".jar").trim_end_matches(".zip");
    trimmed.to_string()
}

fn sanitize_mod_filename(file_name: &str) -> Result<String, String> {
    if file_name.trim().is_empty() {
        return Err("Mod filename is required.".to_string());
    }
    let path = std::path::Path::new(file_name);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Invalid mod filename.".to_string());
    }
    Ok(file_name.to_string())
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
            get_version_manifest_summary,
            get_fabric_loader_versions,
            list_installed_versions,
            list_mods,
            set_mod_enabled,
            delete_mod,
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
