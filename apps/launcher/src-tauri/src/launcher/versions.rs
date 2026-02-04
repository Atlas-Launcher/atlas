use crate::net::http::fetch_json;
use crate::models::{LaunchOptions, ModLoaderKind};
use crate::launcher::{emit, loaders, manifest};
use crate::launcher::error::LauncherError;
use manifest::{VersionData, VersionManifest};
use reqwest::Client;
use std::path::PathBuf;
use tauri::Window;

pub async fn resolve_version_data(
    window: &Window,
    client: &Client,
    manifest: &VersionManifest,
    options: &LaunchOptions,
    game_dir: &PathBuf,
) -> Result<VersionData, LauncherError> {
    let mut version_data = match options.loader.kind {
        ModLoaderKind::Vanilla => {
            let version_id = options
                .version
                .clone()
                .unwrap_or_else(|| manifest.latest.release.clone());
            let version_ref = manifest
                .versions
                .iter()
                .find(|version| version.id == version_id)
                .ok_or_else(|| format!("Version {version_id} not found in manifest"))?;

            emit(
                window,
                "setup",
                format!("Downloading version metadata ({})", version_ref.id),
                None,
                None,
            )?;
            fetch_json::<VersionData>(client, &version_ref.url).await?
        }
        ModLoaderKind::Fabric => {
            let mc_version = options
                .version
                .clone()
                .unwrap_or_else(|| manifest.latest.release.clone());
            emit(
                window,
                "setup",
                format!("Downloading Fabric loader metadata ({mc_version})"),
                None,
                None,
            )?;
            loaders::fabric::fetch_profile(
                client,
                &mc_version,
                options.loader.loader_version.clone(),
            )
            .await?
        }
        ModLoaderKind::NeoForge => {
            let loader_version = options
                .loader
                .loader_version
                .clone()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "NeoForge loader version is required.".to_string())?;
            let version_id = format!("neoforge-{loader_version}");
            emit(
                window,
                "setup",
                format!("Loading NeoForge profile ({version_id})"),
                None,
                None,
            )?;
            loaders::neoforge::ensure_profile(game_dir, &loader_version).await?
        }
    };

    version_data = resolve_inherited_version_data(client, manifest, version_data).await?;
    Ok(version_data)
}

async fn resolve_inherited_version_data(
    client: &Client,
    manifest: &VersionManifest,
    version_data: VersionData,
) -> Result<VersionData, LauncherError> {
    let mut visited = std::collections::HashSet::new();
    let mut chain = vec![version_data];
    let mut next_parent = chain
        .last()
        .and_then(|version| version.inherits_from.clone());

    while let Some(parent_id) = next_parent {
        if !visited.insert(parent_id.clone()) {
            return Err(format!("Version inheritance loop detected at {parent_id}").into());
        }

        let parent_ref = manifest
            .versions
            .iter()
            .find(|version| version.id == parent_id)
            .ok_or_else(|| format!("Parent version {parent_id} not found in manifest"))?;
        let parent_data: VersionData = fetch_json(client, &parent_ref.url).await?;
        next_parent = parent_data.inherits_from.clone();
        chain.push(parent_data);
    }

    let mut merged = chain
        .pop()
        .ok_or_else(|| "Failed to resolve version data.".to_string())?;
    while let Some(overlay) = chain.pop() {
        merged = merge_versions(merged, overlay);
    }

    Ok(merged)
}

fn merge_versions(base: VersionData, overlay: VersionData) -> VersionData {
    let mut libraries = base.libraries;
    libraries.extend(overlay.libraries.clone());

    let arguments = merge_arguments(base.arguments, overlay.arguments.clone());

    VersionData {
        id: overlay.id,
        kind: overlay.kind,
        main_class: if overlay.main_class.trim().is_empty() {
            base.main_class
        } else {
            overlay.main_class
        },
        arguments,
        minecraft_arguments: overlay.minecraft_arguments.or(base.minecraft_arguments),
        asset_index: overlay.asset_index.or(base.asset_index),
        downloads: overlay.downloads.or(base.downloads),
        libraries,
        java_version: overlay.java_version.or(base.java_version),
        inherits_from: None,
    }
}

fn merge_arguments(
    base: Option<manifest::Arguments>,
    overlay: Option<manifest::Arguments>,
) -> Option<manifest::Arguments> {
    match (base, overlay) {
        (Some(mut base), Some(mut overlay)) => {
            base.game.append(&mut overlay.game);
            base.jvm.append(&mut overlay.jvm);
            Some(base)
        }
        (Some(base), None) => Some(base),
        (None, Some(overlay)) => Some(overlay),
        (None, None) => None,
    }
}
