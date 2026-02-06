use crate::launcher::download::download_raw;
use crate::launcher::emit;
use crate::launcher::error::LauncherError;
use crate::launcher::manifest::VersionData;
use crate::models::FabricLoaderVersion;
use crate::net::http::{fetch_json, fetch_text, shared_client, HttpError};
use crate::paths::ensure_dir;
use crate::telemetry;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

const FABRIC_INSTALLER_METADATA_URL: &str =
    "https://maven.fabricmc.net/net/fabricmc/fabric-installer/maven-metadata.xml";
const INSTALLER_LOG_TAIL_LINES: usize = 12;
const FABRIC_INSTALL_MARKER_FILE: &str = "installer_applied.txt";

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

pub async fn fetch_loader_versions(
    client: &Client,
    minecraft_version: &str,
) -> Result<Vec<FabricLoaderVersion>, HttpError> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{minecraft_version}");
    let entries: Vec<FabricLoaderEntry> = fetch_json(client, &url).await?;
    Ok(entries
        .into_iter()
        .map(|entry| FabricLoaderVersion {
            version: entry.loader.version,
            stable: entry.loader.stable,
        })
        .collect())
}

pub async fn fetch_profile(
    client: &Client,
    minecraft_version: &str,
    requested: Option<String>,
) -> Result<VersionData, LauncherError> {
    let loader_version = resolve_loader_version(client, minecraft_version, requested).await?;
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{minecraft_version}/{loader_version}/profile/json"
    );
    Ok(fetch_json::<VersionData>(client, &profile_url).await?)
}

pub async fn ensure_installed(
    window: &tauri::Window,
    game_dir: &Path,
    minecraft_version: &str,
    requested_loader_version: Option<String>,
    java_path: &str,
) -> Result<(), LauncherError> {
    let client = shared_client().clone();
    let loader_version =
        resolve_loader_version(&client, minecraft_version, requested_loader_version).await?;
    let installer_version = fetch_installer_version(&client).await?;

    let marker_dir = game_dir.join("versions").join(format!(
        "fabric-loader-{loader_version}-{minecraft_version}"
    ));
    ensure_dir(&marker_dir)?;
    let marker_path = marker_dir.join(FABRIC_INSTALL_MARKER_FILE);
    if marker_path.exists() {
        emit(
            window,
            "loader",
            format!(
                "Fabric installer already applied (mc {minecraft_version}, loader {loader_version})"
            ),
            None,
            None,
        )?;
        return Ok(());
    }

    let installer_dir = game_dir.join("versions").join("fabric-installer");
    ensure_dir(&installer_dir)?;
    let installer_jar = installer_dir.join(format!("fabric-installer-{installer_version}.jar"));
    let installer_url = format!(
        "https://maven.fabricmc.net/net/fabricmc/fabric-installer/{installer_version}/fabric-installer-{installer_version}.jar"
    );

    if !installer_jar.exists() || installer_jar.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        emit(
            window,
            "loader",
            format!("Downloading Fabric installer.jar ({installer_version})"),
            None,
            None,
        )?;
        download_raw(&client, &installer_url, &installer_jar, None, true).await?;
        emit(
            window,
            "loader",
            format!("Downloaded Fabric installer.jar ({installer_version})"),
            None,
            None,
        )?;
    } else {
        emit(
            window,
            "loader",
            format!("Using cached Fabric installer.jar ({installer_version})"),
            None,
            None,
        )?;
    }

    emit(
        window,
        "loader",
        format!("Running Fabric installer.jar (mc {minecraft_version}, loader {loader_version})"),
        None,
        None,
    )?;
    run_installer(
        window,
        java_path,
        &installer_jar,
        game_dir,
        minecraft_version,
        &loader_version,
    )?;

    std::fs::write(
        &marker_path,
        format!(
            "installer={installer_version}\nminecraft_version={minecraft_version}\nloader_version={loader_version}\n"
        ),
    )
    .map_err(|err| format!("Failed to write Fabric install marker: {err}"))?;

    emit(
        window,
        "loader",
        format!("Fabric installer finished (mc {minecraft_version}, loader {loader_version})"),
        None,
        None,
    )?;
    Ok(())
}

async fn resolve_loader_version(
    client: &Client,
    minecraft_version: &str,
    requested: Option<String>,
) -> Result<String, LauncherError> {
    if let Some(version) = requested {
        if !version.trim().is_empty() {
            return Ok(version.trim().to_string());
        }
    }

    let entries = fetch_loader_versions(client, minecraft_version).await?;
    let chosen = entries
        .iter()
        .find(|entry| entry.stable)
        .or_else(|| entries.first())
        .ok_or_else(|| "No Fabric loader versions found.".to_string())?;
    Ok(chosen.version.clone())
}

async fn fetch_installer_version(client: &Client) -> Result<String, LauncherError> {
    let xml = fetch_text(client, FABRIC_INSTALLER_METADATA_URL).await?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut versions = Vec::new();
    let mut release: Option<String> = None;
    let mut in_version = false;
    let mut in_release = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => match event.name().as_ref() {
                b"version" => in_version = true,
                b"release" => in_release = true,
                _ => {}
            },
            Ok(Event::End(event)) => match event.name().as_ref() {
                b"version" => in_version = false,
                b"release" => in_release = false,
                _ => {}
            },
            Ok(Event::Text(text)) => {
                let value = text
                    .decode()
                    .map_err(|err| format!("Failed to parse Fabric installer metadata: {err}"))?
                    .trim()
                    .to_string();
                if value.is_empty() {
                    buf.clear();
                    continue;
                }
                if in_release {
                    release = Some(value.clone());
                }
                if in_version {
                    versions.push(value);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(format!("Failed to parse Fabric installer metadata: {err}").into())
            }
            _ => {}
        }
        buf.clear();
    }

    if let Some(value) = release.filter(|value| !value.trim().is_empty()) {
        return Ok(value);
    }
    versions
        .last()
        .cloned()
        .ok_or_else(|| "No Fabric installer versions found.".to_string().into())
}

fn run_installer(
    window: &tauri::Window,
    java_path: &str,
    installer_jar: &Path,
    game_dir: &Path,
    minecraft_version: &str,
    loader_version: &str,
) -> Result<(), LauncherError> {
    emit_installer_log(
        window,
        "loader",
        format!(
            "Running Fabric installer against target directory: {}",
            game_dir.display()
        ),
    );

    let mut child = Command::new(java_path)
        .current_dir(game_dir)
        .arg("-jar")
        .arg(installer_jar)
        .arg("client")
        .arg("-dir")
        .arg(game_dir)
        .arg("-mcversion")
        .arg(minecraft_version)
        .arg("-loader")
        .arg(loader_version)
        .arg("-noprofile")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to run Fabric installer: {err}"))?;

    let stdout_tail = Arc::new(Mutex::new(Vec::<String>::new()));
    let stderr_tail = Arc::new(Mutex::new(Vec::<String>::new()));
    let stdout_thread = child.stdout.take().map(|stdout| {
        spawn_installer_stream_forwarder(
            window.clone(),
            "loader-stdout",
            stdout,
            stdout_tail.clone(),
        )
    });
    let stderr_thread = child.stderr.take().map(|stderr| {
        spawn_installer_stream_forwarder(
            window.clone(),
            "loader-stderr",
            stderr,
            stderr_tail.clone(),
        )
    });

    let mut elapsed = 0u64;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                thread::sleep(Duration::from_secs(1));
                elapsed += 1;
                if elapsed % 10 == 0 {
                    let message = format!(
                        "Fabric installer.jar still running (mc {minecraft_version}, loader {loader_version}, {}s)",
                        elapsed
                    );
                    let _ = emit(window, "loader", message.clone(), None, None);
                    emit_installer_log(window, "loader", message);
                }
            }
            Err(err) => {
                let _ = child.kill();
                emit_installer_log(
                    window,
                    "loader",
                    format!("Failed while waiting for Fabric installer: {err}"),
                );
                break None;
            }
        }
    };

    if let Some(thread) = stdout_thread {
        let _ = thread.join();
    }
    if let Some(thread) = stderr_thread {
        let _ = thread.join();
    }

    let Some(status) = status else {
        return Err("Fabric installer terminated unexpectedly."
            .to_string()
            .into());
    };
    if status.success() {
        return Ok(());
    }

    let stderr_sample = tail_to_string(&stderr_tail);
    let stdout_sample = tail_to_string(&stdout_tail);
    let detail = if !stderr_sample.is_empty() {
        stderr_sample
    } else if !stdout_sample.is_empty() {
        stdout_sample
    } else {
        "No installer output captured.".to_string()
    };
    Err(format!(
        "Fabric installer failed with exit code {:?}: {}",
        status.code(),
        detail
    )
    .into())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchLogEvent {
    stream: String,
    message: String,
}

fn emit_installer_log(window: &tauri::Window, stream: &str, message: impl Into<String>) {
    let message = message.into();
    telemetry::info(format!("[modloader:{stream}] {message}"));
    let _ = window.emit(
        "launch://log",
        LaunchLogEvent {
            stream: stream.to_string(),
            message,
        },
    );
}

fn spawn_installer_stream_forwarder<R: Read + Send + 'static>(
    window: tauri::Window,
    stream: &'static str,
    reader: R,
    tail: Arc<Mutex<Vec<String>>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let buffered = BufReader::new(reader);
        for line in buffered.lines() {
            let line = match line {
                Ok(value) => value,
                Err(err) => {
                    emit_installer_log(
                        &window,
                        "loader",
                        format!("Failed to read installer {stream}: {err}"),
                    );
                    break;
                }
            };

            if line.trim().is_empty() {
                continue;
            }
            emit_installer_log(&window, stream, line.clone());
            push_tail_line(&tail, line);
        }
    })
}

fn push_tail_line(tail: &Arc<Mutex<Vec<String>>>, line: String) {
    let Ok(mut lines) = tail.lock() else {
        return;
    };
    lines.push(line);
    if lines.len() > INSTALLER_LOG_TAIL_LINES {
        let overflow = lines.len() - INSTALLER_LOG_TAIL_LINES;
        lines.drain(0..overflow);
    }
}

fn tail_to_string(tail: &Arc<Mutex<Vec<String>>>) -> String {
    let Ok(lines) = tail.lock() else {
        return String::new();
    };
    lines.join(" | ")
}
