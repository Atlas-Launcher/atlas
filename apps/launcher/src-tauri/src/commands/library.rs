use crate::auth;
use crate::config;
use crate::library;
use crate::models::{AtlasRemotePack, FabricLoaderVersion, ModEntry, VersionManifestSummary};
use crate::state::AppState;

#[tauri::command]
pub async fn get_version_manifest_summary() -> Result<VersionManifestSummary, String> {
    library::fetch_version_manifest_summary()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_fabric_loader_versions(
    minecraft_version: String,
) -> Result<Vec<FabricLoaderVersion>, String> {
    library::fetch_fabric_loader_versions(&minecraft_version)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_neoforge_loader_versions() -> Result<Vec<String>, String> {
    library::fetch_neoforge_loader_versions()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn list_installed_versions(game_dir: String) -> Result<Vec<String>, String> {
    library::list_installed_versions(&game_dir).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn list_mods(game_dir: String) -> Result<Vec<ModEntry>, String> {
    library::list_mods(&game_dir).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn set_mod_enabled(game_dir: String, file_name: String, enabled: bool) -> Result<(), String> {
    library::set_mod_enabled(&game_dir, &file_name, enabled).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn delete_mod(game_dir: String, file_name: String) -> Result<(), String> {
    library::delete_mod(&game_dir, &file_name).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn uninstall_instance_data(game_dir: String) -> Result<(), String> {
    library::uninstall_instance_data(&game_dir).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_atlas_remote_packs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AtlasRemotePack>, String> {
    let session = {
        let guard = state
            .atlas_auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        guard
            .clone()
            .ok_or_else(|| "Not signed in to Atlas Hub.".to_string())?
    };

    let refreshed = auth::ensure_fresh_atlas_session(session)
        .await
        .map_err(|err| err.to_string())?;
    auth::save_atlas_session(&refreshed).map_err(|err| err.to_string())?;

    {
        let mut guard = state
            .atlas_auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        *guard = Some(refreshed.clone());
    }

    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);

    library::fetch_atlas_remote_packs(&hub_url, &refreshed.access_token)
        .await
        .map_err(|err| err.to_string())
}
