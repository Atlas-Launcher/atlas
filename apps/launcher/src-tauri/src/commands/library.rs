use crate::auth;
use crate::config;
use crate::library;
use crate::models::{
    AtlasPackSyncResult, AtlasRemotePack, AtlasSession, FabricLoaderVersion, LaunchEvent, ModEntry,
    VersionManifestSummary,
};
use crate::state::AppState;
use crate::telemetry;
use mod_resolver::Provider;
use tauri::Emitter;

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
pub fn uninstall_instance_data(game_dir: String, preserve_saves: Option<bool>) -> Result<(), String> {
    library::uninstall_instance_data(&game_dir, preserve_saves.unwrap_or(false))
        .map_err(|err| err.to_string())
}

async fn get_fresh_atlas_session(
    state: &tauri::State<'_, AppState>,
) -> Result<AtlasSession, String> {
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

    Ok(refreshed)
}

#[tauri::command]
pub async fn resolve_pack_mod(
    state: tauri::State<'_, AppState>,
    source: String,
    query: String,
    loader: String,
    minecraft_version: String,
    desired_version: Option<String>,
    pack_type: String,
) -> Result<mod_resolver::ModEntry, String> {
    let provider =
        Provider::from_short_code(&source).ok_or_else(|| "source must be cf or mr".to_string())?;
    match provider {
        Provider::Modrinth => mod_resolver::resolve(
            provider,
            &query,
            &loader,
            &minecraft_version,
            desired_version.as_deref(),
            &pack_type,
        )
        .await
        .map_err(|err| err.to_string()),
        Provider::CurseForge => {
            let refreshed = get_fresh_atlas_session(&state).await?;
            let settings = state
                .settings
                .lock()
                .map_err(|_| "Settings lock poisoned".to_string())?
                .clone();
            let hub_url = config::resolve_atlas_hub_url(&settings);

            mod_resolver::resolve_curseforge_via_proxy(
                &hub_url,
                &refreshed.access_token,
                &query,
                &loader,
                &minecraft_version,
                desired_version.as_deref(),
                &pack_type,
            )
            .await
            .map_err(|err| err.to_string())
        }
    }
}

#[tauri::command]
pub async fn list_atlas_remote_packs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AtlasRemotePack>, String> {
    let refreshed = get_fresh_atlas_session(&state).await?;

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

#[tauri::command]
pub async fn sync_atlas_pack(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    pack_id: String,
    game_dir: String,
    channel: Option<String>,
) -> Result<AtlasPackSyncResult, String> {
    let refreshed = get_fresh_atlas_session(&state).await?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let hub_url = config::resolve_atlas_hub_url(&settings);

    match library::sync_atlas_pack(
        &window,
        &hub_url,
        &refreshed.access_token,
        &pack_id,
        channel.as_deref(),
        &game_dir,
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(err) => {
            telemetry::error(format!(
                "atlas pack sync failed pack_id={} game_dir={} channel={}: {}",
                pack_id,
                game_dir,
                channel.as_deref().unwrap_or("-"),
                err
            ));
            let _ = window.emit(
                "launch://status",
                LaunchEvent {
                    phase: "atlas-sync".to_string(),
                    message: "Pack update failed".to_string(),
                    current: None,
                    total: None,
                    percent: Some(100),
                },
            );
            Err("Failed to update this Atlas profile. See launcher.log for details.".to_string())
        }
    }
}
