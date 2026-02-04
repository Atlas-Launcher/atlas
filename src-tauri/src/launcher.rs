mod args;
mod download;
mod java;
mod libraries;
mod manifest;

use crate::models::{AuthSession, LaunchEvent, LaunchOptions};
use crate::paths::{ensure_dir, file_exists, normalize_path};
use download::{download_if_needed, download_raw, fetch_json, DOWNLOAD_CONCURRENCY};
use futures::stream::{self, StreamExt};
use java::resolve_java_path;
use libraries::{build_classpath, extract_natives, sync_libraries};
use manifest::{AssetIndexData, Download, VersionData, VersionManifest, VERSION_MANIFEST_URL};
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{Emitter, Window};

struct PreparedMinecraft {
    game_dir: PathBuf,
    assets_dir: PathBuf,
    version_data: VersionData,
    client_jar_path: PathBuf,
    library_paths: Vec<PathBuf>,
    natives_dir: PathBuf,
    java_path: String,
}

pub async fn launch_minecraft(
    window: &Window,
    options: &LaunchOptions,
    session: &AuthSession,
) -> Result<(), String> {
    let prepared = prepare_minecraft(window, options).await?;
    let game_dir = prepared.game_dir;
    let assets_dir = prepared.assets_dir;
    let version_data = prepared.version_data;
    let client_jar_path = prepared.client_jar_path;
    let library_paths = prepared.library_paths;
    let natives_dir = prepared.natives_dir;
    let java_path = prepared.java_path;

    emit(window, "launch", "Preparing JVM arguments", None, None)?;
    let classpath = build_classpath(&library_paths, &client_jar_path);

    let mut replace_map = HashMap::new();
    replace_map.insert("auth_player_name", session.profile.name.clone());
    replace_map.insert("version_name", version_data.id.clone());
    replace_map.insert("game_directory", game_dir.to_string_lossy().to_string());
    replace_map.insert("assets_root", assets_dir.to_string_lossy().to_string());
    replace_map.insert("assets_index_name", version_data.asset_index.id.clone());
    replace_map.insert("auth_uuid", session.profile.id.clone());
    replace_map.insert("auth_access_token", session.access_token.clone());
    replace_map.insert("user_type", "msa".to_string());
    replace_map.insert("version_type", version_data.kind.clone());
    replace_map.insert("classpath", classpath.clone());
    replace_map.insert(
        "natives_directory",
        natives_dir.to_string_lossy().to_string(),
    );
    replace_map.insert("launcher_name", "atlas".to_string());
    replace_map.insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());

    let (mut jvm_args, game_args) = args::build_arguments(&version_data, &replace_map)?;

    let memory = options.memory_mb.max(1024);
    let mem_arg = format!("-Xmx{}M", memory);
    jvm_args.insert(0, mem_arg);
    jvm_args.insert(1, "-Xms512M".into());

    if !jvm_args
        .iter()
        .any(|arg| arg.contains("-Djava.library.path"))
    {
        jvm_args.push(format!(
            "-Djava.library.path={}",
            natives_dir.to_string_lossy()
        ));
    }

    emit(window, "launch", "Spawning Minecraft", None, None)?;
    let mut command = Command::new(java_path);
    command
        .current_dir(&game_dir)
        .args(&jvm_args)
        .arg(&version_data.main_class)
        .args(&game_args);

    command
        .spawn()
        .map_err(|err| format!("Failed to launch Minecraft: {err}"))?;

    emit(window, "launch", "Minecraft process started", None, None)?;
    Ok(())
}

pub async fn download_minecraft_files(
    window: &Window,
    options: &LaunchOptions,
) -> Result<(), String> {
    prepare_minecraft(window, options).await?;
    emit(window, "download", "Minecraft files are ready", None, None)?;
    Ok(())
}

async fn prepare_minecraft(
    window: &Window,
    options: &LaunchOptions,
) -> Result<PreparedMinecraft, String> {
    let client = Client::new();
    let game_dir = normalize_path(&options.game_dir);
    let versions_dir = game_dir.join("versions");
    let libraries_dir = game_dir.join("libraries");
    let assets_dir = game_dir.join("assets");
    ensure_dir(&versions_dir)?;
    ensure_dir(&libraries_dir)?;
    ensure_dir(&assets_dir.join("indexes"))?;
    ensure_dir(&assets_dir.join("objects"))?;

    emit(window, "setup", "Fetching version manifest", None, None)?;
    let manifest: VersionManifest = fetch_json(&client, VERSION_MANIFEST_URL).await?;

    let version_id = options
        .version
        .clone()
        .unwrap_or_else(|| manifest.latest.release.clone());

    let version_ref = manifest
        .versions
        .into_iter()
        .find(|version| version.id == version_id)
        .ok_or_else(|| format!("Version {version_id} not found in manifest"))?;

    emit(
        window,
        "setup",
        format!("Downloading version metadata ({})", version_ref.id),
        None,
        None,
    )?;
    let version_data: VersionData = fetch_json(&client, &version_ref.url).await?;
    let version_folder = versions_dir.join(&version_data.id);
    ensure_dir(&version_folder)?;

    let version_json_path = version_folder.join(format!("{}.json", version_data.id));
    let version_bytes = serde_json::to_vec_pretty(&version_data)
        .map_err(|err| format!("Failed to serialize version metadata: {err}"))?;
    fs::write(&version_json_path, version_bytes)
        .map_err(|err| format!("Failed to write version metadata: {err}"))?;

    emit(window, "client", "Downloading client jar", None, None)?;
    let client_jar_path = version_folder.join(format!("{}.jar", version_data.id));
    download_if_needed(&client, &version_data.downloads.client, &client_jar_path).await?;

    emit(window, "libraries", "Syncing libraries", None, None)?;
    let (library_paths, native_jars) =
        sync_libraries(&client, &libraries_dir, &version_data.libraries, window).await?;

    emit(window, "natives", "Extracting natives", None, None)?;
    let natives_dir = version_folder.join("natives");
    if natives_dir.exists() {
        fs::remove_dir_all(&natives_dir)
            .map_err(|err| format!("Failed to clear natives: {err}"))?;
    }
    ensure_dir(&natives_dir)?;
    for native in native_jars {
        extract_natives(&native, &natives_dir, &version_data.libraries)?;
    }

    emit(window, "assets", "Syncing assets", None, None)?;
    let assets_index_path = assets_dir
        .join("indexes")
        .join(format!("{}.json", version_data.asset_index.id));
    download_if_needed(
        &client,
        &Download {
            path: None,
            url: version_data.asset_index.url.clone(),
            sha1: version_data.asset_index.sha1.clone(),
            size: version_data.asset_index.size,
        },
        &assets_index_path,
    )
    .await?;

    let assets_index_data: AssetIndexData = serde_json::from_slice(
        &fs::read(&assets_index_path)
            .map_err(|err| format!("Failed to read asset index: {err}"))?,
    )
    .map_err(|err| format!("Failed to parse asset index: {err}"))?;

    let total_assets = assets_index_data.objects.len() as u64;
    let mut processed_assets = 0u64;
    let mut asset_jobs: Vec<(String, PathBuf, u64)> = Vec::new();
    for (_name, asset) in assets_index_data.objects.iter() {
        let hash = &asset.hash;
        let sub = &hash[0..2];
        let object_path = assets_dir.join("objects").join(sub).join(hash);
        if file_exists(&object_path) {
            processed_assets += 1;
            if processed_assets % 250 == 0 || processed_assets == total_assets {
                emit(
                    window,
                    "assets",
                    format!("Assets {processed_assets}/{total_assets}"),
                    Some(processed_assets),
                    Some(total_assets),
                )?;
            }
            continue;
        }
        let url = format!("https://resources.download.minecraft.net/{}/{}", sub, hash);
        asset_jobs.push((url, object_path, asset.size));
    }

    if !asset_jobs.is_empty() {
        let mut stream = stream::iter(asset_jobs.into_iter().map(|(url, path, size)| {
            let client = client.clone();
            async move { download_raw(&client, &url, &path, Some(size), true).await }
        }))
        .buffer_unordered(DOWNLOAD_CONCURRENCY);

        while let Some(result) = stream.next().await {
            result?;
            processed_assets += 1;
            if processed_assets % 250 == 0 || processed_assets == total_assets {
                emit(
                    window,
                    "assets",
                    format!("Assets {processed_assets}/{total_assets}"),
                    Some(processed_assets),
                    Some(total_assets),
                )?;
            }
        }
    }

    let java_path = resolve_java_path(window, &game_dir, &version_data, &options.java_path).await?;

    Ok(PreparedMinecraft {
        game_dir,
        assets_dir,
        version_data,
        client_jar_path,
        library_paths,
        natives_dir,
        java_path,
    })
}

pub(crate) fn emit(
    window: &Window,
    phase: &str,
    message: impl Into<String>,
    current: Option<u64>,
    total: Option<u64>,
) -> Result<(), String> {
    window
        .emit(
            "launch://status",
            LaunchEvent {
                phase: phase.into(),
                message: message.into(),
                current,
                total,
                percent: None,
            },
        )
        .map_err(|err| format!("Emit failed: {err}"))
}

#[cfg(test)]
mod tests;
