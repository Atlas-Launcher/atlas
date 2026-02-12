mod atlas_sync;
mod error;

use crate::launcher::manifest::VersionManifest;
use crate::models::{
    AtlasPackSyncResult, AtlasRemotePack, FabricLoaderVersion, ModEntry, VersionManifestSummary,
    VersionSummary,
};
use crate::net::http::{fetch_json_shared, shared_client};
use crate::paths;
use atlas_client::hub::HubClient;
use error::LibraryError;
use std::collections::HashSet;
use std::fs;
use std::path::Component;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Window;

pub async fn fetch_version_manifest_summary() -> Result<VersionManifestSummary, LibraryError> {
    let manifest: VersionManifest =
        fetch_json_shared(crate::launcher::manifest::VERSION_MANIFEST_URL).await?;
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

pub async fn fetch_fabric_loader_versions(
    minecraft_version: &str,
) -> Result<Vec<FabricLoaderVersion>, LibraryError> {
    let client = shared_client();
    Ok(crate::launcher::loaders::fabric::fetch_loader_versions(client, minecraft_version).await?)
}

pub async fn fetch_neoforge_loader_versions() -> Result<Vec<String>, LibraryError> {
    let client = shared_client();
    Ok(crate::launcher::loaders::neoforge::fetch_loader_versions(client).await?)
}

pub async fn fetch_atlas_remote_packs(
    atlas_hub_url: &str,
    access_token: &str,
) -> Result<Vec<AtlasRemotePack>, LibraryError> {
    let mut hub =
        HubClient::new(atlas_hub_url).map_err(|err| LibraryError::Message(err.to_string()))?;
    hub.set_token(access_token.to_string());
    let packs = hub
        .list_launcher_packs()
        .await
        .map_err(|err| LibraryError::Message(err.to_string()))?;
    let mut seen_pack_ids = HashSet::new();
    let mut remote_packs = Vec::new();
    for pack in packs {
        if !seen_pack_ids.insert(pack.pack_id.clone()) {
            continue;
        }
        remote_packs.push(AtlasRemotePack {
            pack_id: pack.pack_id,
            pack_name: pack.pack_name,
            pack_slug: pack.pack_slug,
            access_level: "unknown".to_string(),
            channel: pack.channel,
            build_id: None,
            build_version: None,
            artifact_key: None,
            minecraft_version: None,
            modloader: None,
            modloader_version: None,
        });
    }
    Ok(remote_packs)
}

pub async fn sync_atlas_pack(
    window: &Window,
    atlas_hub_url: &str,
    access_token: &str,
    pack_id: &str,
    channel: Option<&str>,
    game_dir: &str,
) -> Result<AtlasPackSyncResult, LibraryError> {
    atlas_sync::sync_atlas_pack(
        window,
        atlas_hub_url,
        access_token,
        pack_id,
        channel,
        game_dir,
    )
    .await
}

pub fn list_installed_versions(game_dir: &str) -> Result<Vec<String>, LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let minecraft_dir = minecraft_dir_for_instance(&base_dir);
    let versions_dir = minecraft_dir.join("versions");
    let mut versions = Vec::new();
    if versions_dir.exists() {
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
    }

    // Atlas-managed channel installs may not have a Minecraft versions tree yet.
    // Treat sync metadata as an installed marker so launcher UI can show installed state.
    if versions.is_empty() && base_dir.join("last_updated.toml").exists() {
        versions.push("atlas-managed".to_string());
    }

    versions.sort();
    Ok(versions)
}

pub fn list_mods(game_dir: &str) -> Result<Vec<ModEntry>, LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let mods_dir = minecraft_dir_for_instance(&base_dir).join("mods");
    paths::ensure_dir(&mods_dir)?;

    let mut mods = Vec::new();
    let entries =
        fs::read_dir(&mods_dir).map_err(|err| format!("Failed to read mods dir: {err}"))?;
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
        let metadata =
            fs::metadata(&path).map_err(|err| format!("Failed to read mod metadata: {err}"))?;
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
    mods.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    Ok(mods)
}

pub fn set_mod_enabled(game_dir: &str, file_name: &str, enabled: bool) -> Result<(), LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let mods_dir = minecraft_dir_for_instance(&base_dir).join("mods");
    paths::ensure_dir(&mods_dir)?;

    let safe_name = sanitize_mod_filename(file_name)?;
    let current_path = mods_dir.join(&safe_name);
    if !current_path.exists() {
        return Err(format!("Mod {safe_name} not found.").into());
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
        return Err(format!("Cannot toggle mod. Target file already exists: {target_name}").into());
    }

    fs::rename(&current_path, &target_path)
        .map_err(|err| format!("Failed to rename mod: {err}"))?;
    Ok(())
}

pub fn delete_mod(game_dir: &str, file_name: &str) -> Result<(), LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let mods_dir = minecraft_dir_for_instance(&base_dir).join("mods");
    paths::ensure_dir(&mods_dir)?;

    let safe_name = sanitize_mod_filename(file_name)?;
    let path = mods_dir.join(&safe_name);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| format!("Failed to delete mod: {err}"))?;
    Ok(())
}

pub fn uninstall_instance_data(game_dir: &str, preserve_saves: bool) -> Result<(), LibraryError> {
    let trimmed = game_dir.trim();
    if trimmed.is_empty() {
        return Err("Game directory is required.".to_string().into());
    }

    let base_dir = paths::normalize_path(trimmed);
    if !base_dir.exists() {
        return Ok(());
    }

    let segments: Vec<String> = base_dir
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string().to_lowercase()),
            _ => None,
        })
        .collect();

    let in_instances_dir = segments
        .iter()
        .rposition(|segment| segment == "instances")
        .is_some_and(|index| index + 1 < segments.len());
    if !in_instances_dir {
        return Err(
            "Refusing to uninstall outside the launcher instances directory."
                .to_string()
                .into(),
        );
    }

    if !preserve_saves {
        fs::remove_dir_all(&base_dir)
            .map_err(|err| format!("Failed to remove instance data: {err}"))?;
        return Ok(());
    }

    let minecraft_dir = minecraft_dir_for_instance(&base_dir);
    let saves_path = minecraft_dir.join("saves");
    let mut preserved_saves_path = None;
    if saves_path.exists() {
        let parent = base_dir
            .parent()
            .ok_or_else(|| "Instance directory has no parent.".to_string())?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| format!("Failed to read system clock: {err}"))?
            .as_millis();
        let mut candidate = parent.join(format!(".atlas-preserve-saves-{stamp}"));
        let mut suffix = 0usize;
        while candidate.exists() {
            suffix += 1;
            candidate = parent.join(format!(".atlas-preserve-saves-{stamp}-{suffix}"));
        }
        fs::rename(&saves_path, &candidate).map_err(|err| {
            format!(
                "Failed to preserve saves directory {}: {err}",
                saves_path.display()
            )
        })?;
        preserved_saves_path = Some(candidate);
    }

    if let Err(err) = fs::remove_dir_all(&base_dir) {
        if let Some(path) = preserved_saves_path {
            let _ = fs::rename(path, saves_path);
        }
        return Err(format!("Failed to remove instance data: {err}").into());
    }

    fs::create_dir_all(&base_dir)
        .map_err(|err| format!("Failed to recreate instance data directory: {err}"))?;
    fs::create_dir_all(&minecraft_dir).map_err(|err| {
        format!(
            "Failed to recreate Minecraft data directory {}: {err}",
            minecraft_dir.display()
        )
    })?;

    if let Some(path) = preserved_saves_path {
        let restored_saves_path = minecraft_dir.join("saves");
        fs::rename(&path, &restored_saves_path).map_err(|err| {
            format!(
                "Failed to restore saves directory to {}: {err}",
                restored_saves_path.display()
            )
        })?;
    }

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

fn sanitize_mod_filename(file_name: &str) -> Result<String, LibraryError> {
    if file_name.trim().is_empty() {
        return Err("Mod filename is required.".to_string().into());
    }
    let path = std::path::Path::new(file_name);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("Invalid mod filename.".to_string().into());
    }
    Ok(file_name.to_string())
}

fn minecraft_dir_for_instance(base_dir: &std::path::Path) -> std::path::PathBuf {
    let modern_dir = base_dir.join(".minecraft");
    if modern_dir.exists() {
        return modern_dir;
    }

    // Backward compatibility for legacy installs that wrote files directly into the instance dir.
    if base_dir.join("versions").exists() || base_dir.join("mods").exists() {
        return base_dir.to_path_buf();
    }

    modern_dir
}
