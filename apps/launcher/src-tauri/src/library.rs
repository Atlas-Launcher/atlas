mod error;

use crate::launcher::manifest::VersionManifest;
use crate::models::{
    AtlasRemotePack, FabricLoaderVersion, ModEntry, VersionManifestSummary, VersionSummary,
};
use crate::net::http::{fetch_json_shared, shared_client, HttpClient, ReqwestHttpClient};
use crate::paths;
use error::LibraryError;
use serde::Deserialize;
use std::fs;
use std::path::Component;

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AtlasRemotePackResponse {
    packs: Vec<AtlasRemotePack>,
}

pub async fn fetch_atlas_remote_packs(
    atlas_hub_url: &str,
    access_token: &str,
) -> Result<Vec<AtlasRemotePack>, LibraryError> {
    let endpoint = format!(
        "{}/api/launcher/packs",
        atlas_hub_url.trim_end_matches('/')
    );
    let http = ReqwestHttpClient::new();
    let response = http
        .get_json::<AtlasRemotePackResponse>(&endpoint, Some(access_token))
        .await?;
    Ok(response.packs)
}

pub fn list_installed_versions(game_dir: &str) -> Result<Vec<String>, LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let versions_dir = base_dir.join("versions");
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    let entries =
        fs::read_dir(&versions_dir).map_err(|err| format!("Failed to read versions dir: {err}"))?;
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

pub fn list_mods(game_dir: &str) -> Result<Vec<ModEntry>, LibraryError> {
    let base_dir = paths::normalize_path(game_dir);
    let mods_dir = base_dir.join("mods");
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
    let mods_dir = base_dir.join("mods");
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
    let mods_dir = base_dir.join("mods");
    paths::ensure_dir(&mods_dir)?;

    let safe_name = sanitize_mod_filename(file_name)?;
    let path = mods_dir.join(&safe_name);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path).map_err(|err| format!("Failed to delete mod: {err}"))?;
    Ok(())
}

pub fn uninstall_instance_data(game_dir: &str) -> Result<(), LibraryError> {
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

    fs::remove_dir_all(&base_dir).map_err(|err| format!("Failed to remove instance data: {err}"))?;
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
