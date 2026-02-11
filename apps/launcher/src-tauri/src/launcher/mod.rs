mod args;
pub(crate) mod download;
mod error;
pub(crate) mod java;
mod libraries;
pub(crate) mod loaders;
pub(crate) mod manifest;
mod versions;

use crate::models::{AuthSession, LaunchEvent, LaunchOptions, ModLoaderKind};
use crate::net::http::{fetch_json, shared_client};
use crate::paths::{ensure_dir, file_exists, normalize_path};
use download::{download_if_needed, download_raw, DOWNLOAD_CONCURRENCY};
use error::LauncherError;
use futures::stream::{self, StreamExt};
use java::resolve_java_path;
use libraries::{build_classpath, extract_natives, sync_libraries};
use manifest::{AssetIndexData, Download, VersionManifest, VERSION_MANIFEST_URL};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Window};

const WINDOW_DETECTION_TIMEOUT: Duration = Duration::from_secs(120);
type LaunchLogSink = Arc<Mutex<std::fs::File>>;

struct PreparedMinecraft {
    instance_dir: PathBuf,
    game_dir: PathBuf,
    assets_dir: PathBuf,
    version_data: manifest::VersionData,
    client_jar_path: PathBuf,
    library_paths: Vec<PathBuf>,
    natives_dir: PathBuf,
    java_path: String,
}

pub async fn launch_minecraft(
    window: &Window,
    options: &LaunchOptions,
    session: &AuthSession,
) -> Result<(), LauncherError> {
    let prepared = prepare_minecraft(window, options).await?;
    let instance_dir = prepared.instance_dir;
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
    replace_map.insert(
        "library_directory",
        game_dir.join("libraries").to_string_lossy().to_string(),
    );
    replace_map.insert(
        "classpath_separator",
        if cfg!(target_os = "windows") {
            ";".to_string()
        } else {
            ":".to_string()
        },
    );
    replace_map.insert("assets_root", assets_dir.to_string_lossy().to_string());
    replace_map.insert("game_assets", assets_dir.to_string_lossy().to_string());
    let asset_index_id = version_data
        .asset_index
        .as_ref()
        .ok_or_else(|| "Missing asset index after resolving version".to_string())?
        .id
        .clone();
    replace_map.insert("assets_index_name", asset_index_id);
    replace_map.insert("auth_uuid", session.profile.id.clone());
    replace_map.insert("auth_access_token", session.access_token.clone());
    replace_map.insert("auth_session", session.access_token.clone());
    replace_map.insert("auth_xuid", String::new());
    replace_map.insert("clientid", session.client_id.clone());
    replace_map.insert("user_properties", "{}".to_string());
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
    let mut unresolved = args::unresolved_tokens(&jvm_args);
    unresolved.extend(args::unresolved_tokens(&game_args));
    unresolved.sort();
    unresolved.dedup();
    if !unresolved.is_empty() {
        return Err(format!(
            "Launch metadata contains unresolved placeholders: {}",
            unresolved.join(", ")
        )
        .into());
    }

    let memory = options.memory_mb.max(1024);
    let mem_arg = format!("-Xmx{}M", memory);
    jvm_args.insert(0, mem_arg);
    jvm_args.insert(1, "-Xms512M".into());
    jvm_args.extend(args::split_jvm_args(&options.jvm_args));

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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&jvm_args)
        .arg(&version_data.main_class)
        .args(&game_args);

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to launch Minecraft: {err}"))?;

    let window_visible = Arc::new(AtomicBool::new(false));
    let launch_terminal = Arc::new(AtomicBool::new(false));
    let launch_log_sink = init_launch_log_sink(&instance_dir);
    if launch_log_sink.is_none() {
        let _ = emit_log(
            window,
            "system",
            "Failed to initialize latest_launch.log; continuing without file logging.",
        );
    } else {
        append_launch_log(
            &launch_log_sink,
            "system",
            "Launch started. Streaming Minecraft logs.",
        );
    }

    if let Some(stdout) = child.stdout.take() {
        spawn_minecraft_log_forwarder(
            window.clone(),
            stdout,
            "stdout",
            window_visible.clone(),
            launch_terminal.clone(),
            launch_log_sink.clone(),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_minecraft_log_forwarder(
            window.clone(),
            stderr,
            "stderr",
            window_visible.clone(),
            launch_terminal.clone(),
            launch_log_sink.clone(),
        );
    }

    spawn_minecraft_process_watcher(
        window.clone(),
        child,
        window_visible.clone(),
        launch_terminal.clone(),
        launch_log_sink.clone(),
    );
    spawn_window_visible_timeout_failure(
        window.clone(),
        window_visible,
        launch_terminal,
        launch_log_sink,
    );

    emit(
        window,
        "launch",
        "Minecraft process started; waiting for game window",
        None,
        None,
    )?;
    Ok(())
}

pub async fn download_minecraft_files(
    window: &Window,
    options: &LaunchOptions,
) -> Result<(), LauncherError> {
    prepare_minecraft(window, options).await?;
    emit(window, "download", "Minecraft files are ready", None, None)?;
    Ok(())
}

async fn prepare_minecraft(
    window: &Window,
    options: &LaunchOptions,
) -> Result<PreparedMinecraft, LauncherError> {
    let client = shared_client().clone();
    let instance_dir = normalize_path(&options.game_dir);
    ensure_dir(&instance_dir)?;
    let game_dir = instance_dir.join(".minecraft");
    ensure_dir(&game_dir)?;
    let versions_dir = game_dir.join("versions");
    let libraries_dir = game_dir.join("libraries");
    let assets_dir = game_dir.join("assets");
    ensure_dir(&versions_dir)?;
    ensure_dir(&libraries_dir)?;
    ensure_dir(&assets_dir.join("indexes"))?;
    ensure_dir(&assets_dir.join("objects"))?;

    emit(window, "setup", "Fetching version manifest", None, None)?;
    let manifest: VersionManifest = fetch_json(&client, VERSION_MANIFEST_URL).await?;

    let version_data =
        versions::resolve_version_data(window, &client, &manifest, options, &game_dir).await?;
    let java_path =
        resolve_java_path(window, &instance_dir, &version_data, &options.java_path).await?;

    let version_folder = versions_dir.join(&version_data.id);
    ensure_dir(&version_folder)?;

    let version_json_path = version_folder.join(format!("{}.json", version_data.id));
    let version_bytes = serde_json::to_vec_pretty(&version_data)
        .map_err(|err| format!("Failed to serialize version metadata: {err}"))?;
    fs::write(&version_json_path, version_bytes)
        .map_err(|err| format!("Failed to write version metadata: {err}"))?;

    let downloads = version_data
        .downloads
        .as_ref()
        .ok_or_else(|| "Missing download metadata after resolving version".to_string())?;
    let client_download = downloads.client.clone();
    emit(window, "client", "Downloading client jar", None, None)?;
    let client_jar_path = version_folder.join(format!("{}.jar", version_data.id));
    download_if_needed(&client, &client_download, &client_jar_path).await?;

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
    let asset_index = version_data
        .asset_index
        .as_ref()
        .ok_or_else(|| "Missing asset index after resolving version".to_string())?;
    let assets_index_path = assets_dir
        .join("indexes")
        .join(format!("{}.json", asset_index.id));
    download_if_needed(
        &client,
        &Download {
            path: None,
            url: asset_index.url.clone(),
            sha1: asset_index.sha1.clone(),
            size: asset_index.size,
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

    if matches!(options.loader.kind, ModLoaderKind::Fabric) {
        let minecraft_version = options
            .version
            .clone()
            .unwrap_or_else(|| manifest.latest.release.clone());
        emit(
            window,
            "setup",
            format!("Installing Fabric loader ({minecraft_version})"),
            None,
            None,
        )?;
        loaders::fabric::ensure_installed(
            window,
            &game_dir,
            &minecraft_version,
            options.loader.loader_version.clone(),
            &java_path,
        )
        .await?;
    }

    if matches!(options.loader.kind, ModLoaderKind::NeoForge) {
        let loader_version = options
            .loader
            .loader_version
            .clone()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "NeoForge loader version is required.".to_string())?;
        emit(
            window,
            "setup",
            format!("Installing NeoForge loader ({loader_version})"),
            None,
            None,
        )?;
        loaders::neoforge::ensure_installed(window, &game_dir, &loader_version, &java_path).await?;
    }

    // Fabric/Forge-family installers may rewrite the version directory. Ensure the launch jar
    // still exists at versions/<version-id>/<version-id>.jar before we build classpath.
    if !file_exists(&client_jar_path) {
        emit(
            window,
            "client",
            "Client jar missing after loader install; restoring",
            None,
            None,
        )?;
        download_if_needed(&client, &client_download, &client_jar_path).await?;
    }
    if !file_exists(&client_jar_path) {
        return Err(format!(
            "Client jar is missing after prepare: {}",
            client_jar_path.display()
        )
        .into());
    }

    Ok(PreparedMinecraft {
        instance_dir,
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
) -> Result<(), LauncherError> {
    emit_with_percent(window, phase, message, current, total, None)
}

fn emit_with_percent(
    window: &Window,
    phase: &str,
    message: impl Into<String>,
    current: Option<u64>,
    total: Option<u64>,
    percent: Option<u64>,
) -> Result<(), LauncherError> {
    window
        .emit(
            "launch://status",
            LaunchEvent {
                phase: phase.into(),
                message: message.into(),
                current,
                total,
                percent,
            },
        )
        .map_err(|err| format!("Emit failed: {err}").into())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchLogEvent {
    stream: String,
    message: String,
}

fn emit_log(
    window: &Window,
    stream: &str,
    message: impl Into<String>,
) -> Result<(), LauncherError> {
    window
        .emit(
            "launch://log",
            LaunchLogEvent {
                stream: stream.to_string(),
                message: message.into(),
            },
        )
        .map_err(|err| format!("Emit failed: {err}").into())
}

fn init_launch_log_sink(game_dir: &std::path::Path) -> Option<LaunchLogSink> {
    let path = game_dir.join("latest_launch.log");
    if let Some(parent) = path.parent() {
        if ensure_dir(parent).is_err() {
            return None;
        }
    }

    match std::fs::File::create(path) {
        Ok(file) => Some(Arc::new(Mutex::new(file))),
        Err(_) => None,
    }
}

fn append_launch_log(log_sink: &Option<LaunchLogSink>, stream: &str, message: &str) {
    let Some(log_sink) = log_sink else {
        return;
    };

    let mut file = match log_sink.lock() {
        Ok(handle) => handle,
        Err(_) => return,
    };
    let _ = writeln!(file, "[{stream}] {message}");
    let _ = file.flush();
}

fn spawn_minecraft_log_forwarder<R: Read + Send + 'static>(
    window: Window,
    reader: R,
    stream: &'static str,
    window_visible: Arc<AtomicBool>,
    launch_terminal: Arc<AtomicBool>,
    launch_log_sink: Option<LaunchLogSink>,
) {
    std::thread::spawn(move || {
        let buffered = BufReader::new(reader);
        for line in buffered.lines() {
            let line = match line {
                Ok(value) => value,
                Err(err) => {
                    let message = format!("Minecraft {stream} read error: {err}");
                    let _ = emit_log(&window, "system", message.clone());
                    append_launch_log(&launch_log_sink, "system", &message);
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let _ = emit_log(&window, stream, line.clone());
            append_launch_log(&launch_log_sink, stream, &line);
            if indicates_window_visible(&line) && !window_visible.swap(true, Ordering::SeqCst) {
                if !launch_terminal.swap(true, Ordering::SeqCst) {
                    let _ = emit_with_percent(
                        &window,
                        "launch",
                        "Minecraft window is on-screen",
                        None,
                        None,
                        Some(100),
                    );
                }
            }
        }
    });
}

fn spawn_minecraft_process_watcher(
    window: Window,
    mut child: std::process::Child,
    window_visible: Arc<AtomicBool>,
    launch_terminal: Arc<AtomicBool>,
    launch_log_sink: Option<LaunchLogSink>,
) {
    std::thread::spawn(move || match child.wait() {
        Ok(status) => {
            let status_line = if let Some(code) = status.code() {
                format!("Minecraft process exited with code {code}.")
            } else {
                "Minecraft process exited.".to_string()
            };
            let _ = emit_log(&window, "system", status_line.clone());
            append_launch_log(&launch_log_sink, "system", &status_line);

            if !window_visible.load(Ordering::SeqCst)
                && !launch_terminal.swap(true, Ordering::SeqCst)
            {
                let message = format!("Launch failed: {status_line}");
                let _ =
                    emit_with_percent(&window, "launch", message.clone(), None, None, Some(100));
                append_launch_log(&launch_log_sink, "system", &message);
            }
        }
        Err(err) => {
            let message = format!("Failed to monitor Minecraft process: {err}");
            let _ = emit_log(&window, "system", message.clone());
            append_launch_log(&launch_log_sink, "system", &message);
            if !window_visible.load(Ordering::SeqCst)
                && !launch_terminal.swap(true, Ordering::SeqCst)
            {
                let launch_message = format!("Launch failed: {message}");
                let _ = emit_with_percent(
                    &window,
                    "launch",
                    launch_message.clone(),
                    None,
                    None,
                    Some(100),
                );
                append_launch_log(&launch_log_sink, "system", &launch_message);
            }
        }
    });
}

fn spawn_window_visible_timeout_failure(
    window: Window,
    window_visible: Arc<AtomicBool>,
    launch_terminal: Arc<AtomicBool>,
    launch_log_sink: Option<LaunchLogSink>,
) {
    std::thread::spawn(move || {
        std::thread::sleep(WINDOW_DETECTION_TIMEOUT);
        if !window_visible.load(Ordering::SeqCst) && !launch_terminal.swap(true, Ordering::SeqCst) {
            let message = format!(
                "Launch failed: Minecraft window was not detected within {} seconds.",
                WINDOW_DETECTION_TIMEOUT.as_secs()
            );
            let _ = emit_log(&window, "system", message.clone());
            append_launch_log(&launch_log_sink, "system", &message);
            let _ = emit_with_percent(&window, "launch", message, None, None, Some(100));
        }
    });
}

fn indicates_window_visible(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("render thread")
        || (lower.contains("window") && lower.contains("created"))
        || (lower.contains("display")
            && (lower.contains("initialized") || lower.contains("created")))
}

#[cfg(test)]
mod tests;
