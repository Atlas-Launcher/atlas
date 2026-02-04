use crate::library;
use crate::models::{FabricLoaderVersion, ModEntry, VersionManifestSummary};

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
