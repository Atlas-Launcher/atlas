use crate::launcher::download::download_raw;
use crate::launcher::emit;
use crate::launcher::error::LauncherError;
use crate::launcher::manifest::VersionData;
use crate::net::http::{fetch_text, shared_client, HttpError};
use crate::paths::ensure_dir;
use crate::telemetry;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use serde::Serialize;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Emitter;
use zip::ZipArchive;

const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
const INSTALL_MARKER_FILE: &str = "installer_applied.txt";
const INSTALLER_LOG_TAIL_LINES: usize = 12;

pub async fn fetch_loader_versions(client: &Client) -> Result<Vec<String>, HttpError> {
    let xml = fetch_text(client, NEOFORGE_METADATA_URL).await?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut versions = Vec::new();
    let mut in_versions = false;
    let mut in_version = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => match event.name().as_ref() {
                b"versions" => in_versions = true,
                b"version" if in_versions => in_version = true,
                _ => {}
            },
            Ok(Event::End(event)) => match event.name().as_ref() {
                b"versions" => in_versions = false,
                b"version" => in_version = false,
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if in_versions && in_version {
                    let value = text.decode().map_err(|err| HttpError::ParseMessage {
                        message: err.to_string(),
                        body: xml.clone(),
                    })?;
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        versions.push(trimmed.to_string());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(HttpError::ParseMessage {
                    message: err.to_string(),
                    body: xml.clone(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    versions.reverse();
    Ok(versions)
}

pub async fn ensure_profile(
    window: &tauri::Window,
    game_dir: &Path,
    loader_version: &str,
) -> Result<VersionData, LauncherError> {
    let loader_version = loader_version.trim();
    if loader_version.is_empty() {
        return Err("NeoForge loader version is required.".to_string().into());
    }
    let version_id = format!("neoforge-{loader_version}");
    let (_version_dir, installer_path, version_json_path) =
        ensure_installer_jar(window, game_dir, loader_version, &version_id).await?;

    if version_json_path.exists() {
        emit(
            window,
            "loader",
            format!("NeoForge profile metadata is ready ({loader_version})"),
            None,
            None,
        )?;
        return read_profile(&version_json_path);
    }

    emit(
        window,
        "loader",
        format!("Extracting NeoForge profile metadata ({loader_version})"),
        None,
        None,
    )?;
    let version_bytes = extract_version_json(&installer_path)?;
    std::fs::write(&version_json_path, &version_bytes)
        .map_err(|err| format!("Failed to write NeoForge profile: {err}"))?;
    emit(
        window,
        "loader",
        format!("NeoForge profile metadata extracted ({loader_version})"),
        None,
        None,
    )?;
    read_profile(&version_json_path)
}

pub async fn ensure_installed(
    window: &tauri::Window,
    game_dir: &Path,
    loader_version: &str,
    java_path: &str,
) -> Result<(), LauncherError> {
    let loader_version = loader_version.trim();
    if loader_version.is_empty() {
        return Err("NeoForge loader version is required.".to_string().into());
    }
    let version_id = format!("neoforge-{loader_version}");
    let (version_dir, installer_path, version_json_path) =
        ensure_installer_jar(window, game_dir, loader_version, &version_id).await?;
    let marker_path = version_dir.join(INSTALL_MARKER_FILE);

    if marker_path.exists() && version_json_path.exists() {
        emit(
            window,
            "loader",
            format!("NeoForge installer already applied ({loader_version})"),
            None,
            None,
        )?;
        return Ok(());
    }

    ensure_launcher_profile(window, game_dir)?;

    emit(
        window,
        "loader",
        format!("Running NeoForge installer.jar ({loader_version})"),
        None,
        None,
    )?;
    run_installer(window, java_path, &installer_path, game_dir, loader_version)?;
    std::fs::write(
        &marker_path,
        format!("loader={loader_version}\ninstance={}\n", game_dir.display()),
    )
    .map_err(|err| format!("Failed to write NeoForge install marker: {err}"))?;
    emit(
        window,
        "loader",
        format!("NeoForge installer finished ({loader_version})"),
        None,
        None,
    )?;
    Ok(())
}

async fn ensure_installer_jar(
    window: &tauri::Window,
    game_dir: &Path,
    loader_version: &str,
    version_id: &str,
) -> Result<(PathBuf, PathBuf, PathBuf), LauncherError> {
    let version_dir = game_dir.join("versions").join(version_id);
    let version_json_path = version_dir.join(format!("{version_id}.json"));
    ensure_dir(&version_dir)?;

    let installer_path = version_dir.join(format!("{version_id}-installer.jar"));
    let installer_url = installer_url(loader_version);
    let client = shared_client().clone();
    if !installer_path.exists() || installer_path.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        emit(
            window,
            "loader",
            format!("Downloading NeoForge installer.jar ({loader_version})"),
            None,
            None,
        )?;
        download_raw(&client, &installer_url, &installer_path, None, true).await?;
        emit(
            window,
            "loader",
            format!("Downloaded NeoForge installer.jar ({loader_version})"),
            None,
            None,
        )?;
    } else {
        emit(
            window,
            "loader",
            format!("Using cached NeoForge installer.jar ({loader_version})"),
            None,
            None,
        )?;
    }
    Ok((version_dir, installer_path, version_json_path))
}

fn installer_url(loader_version: &str) -> String {
    format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar")
}

fn run_installer(
    window: &tauri::Window,
    java_path: &str,
    installer_path: &Path,
    game_dir: &Path,
    loader_version: &str,
) -> Result<(), LauncherError> {
    let game_dir_arg = game_dir.to_string_lossy().to_string();
    let attempts: Vec<Vec<String>> =
        vec![vec!["--installClient".to_string(), game_dir_arg.clone()]];

    let mut failures: Vec<String> = Vec::new();
    emit_installer_log(
        window,
        "loader",
        format!(
            "Running installer against target directory: {}",
            game_dir.display()
        ),
    );
    let attempt_total = attempts.len();
    for (attempt_index, args) in attempts.into_iter().enumerate() {
        let _ = emit(
            window,
            "loader",
            format!(
                "NeoForge installer attempt {}/{}",
                attempt_index + 1,
                attempt_total
            ),
            None,
            None,
        );
        let mut child = Command::new(java_path)
            .current_dir(game_dir)
            .arg("-jar")
            .arg(installer_path)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("Failed to run NeoForge installer: {err}"))?;

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
                            "NeoForge installer.jar still running ({loader_version}, {}s)",
                            elapsed
                        );
                        let _ = emit(window, "loader", message.clone(), None, None);
                        emit_installer_log(window, "loader", message);
                    }
                }
                Err(err) => {
                    failures.push(format!(
                        "args {:?}: Failed to wait for installer: {err}",
                        args
                    ));
                    let _ = child.kill();
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
            continue;
        };

        if status.success() {
            emit_installer_log(
                window,
                "loader",
                format!("NeoForge installer attempt {} succeeded", attempt_index + 1),
            );
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
        failures.push(format!(
            "args {:?}: exit code {:?}. {}",
            args,
            status.code(),
            detail
        ));
    }

    Err(format!(
        "NeoForge/Forge installer failed for instance {}. Attempts: {}",
        game_dir.display(),
        failures.join(" | ")
    )
    .into())
}

fn ensure_launcher_profile(window: &tauri::Window, game_dir: &Path) -> Result<(), LauncherError> {
    let launcher_profile_path = game_dir.join("launcher_profiles.json");
    if launcher_profile_path.exists() {
        return Ok(());
    }

    let payload = serde_json::json!({
        "profiles": {
            "atlas": {
                "name": "Atlas",
                "type": "custom",
                "lastVersionId": "latest-release"
            }
        },
        "selectedProfile": "atlas",
        "authenticationDatabase": {},
        "settings": {},
        "version": 3
    });
    let bytes = serde_json::to_vec_pretty(&payload)
        .map_err(|err| format!("Failed to serialize launcher_profiles.json: {err}"))?;
    std::fs::write(&launcher_profile_path, bytes).map_err(|err| {
        format!(
            "Failed to create launcher profile {}: {err}",
            launcher_profile_path.display()
        )
    })?;

    let _ = emit(
        window,
        "loader",
        "Created launcher_profiles.json for installer compatibility",
        None,
        None,
    );
    emit_installer_log(
        window,
        "loader",
        format!(
            "Created installer-compatible launcher profile: {}",
            launcher_profile_path.display()
        ),
    );
    Ok(())
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

fn extract_version_json(installer_path: &Path) -> Result<Vec<u8>, LauncherError> {
    let file =
        std::fs::File::open(installer_path).map_err(|err| format!("Open installer: {err}"))?;
    let mut archive = ZipArchive::new(file).map_err(|err| format!("Read installer jar: {err}"))?;

    if let Ok(mut entry) = archive.by_name("version.json") {
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Read installer version.json: {err}"))?;
        return Ok(buffer);
    }

    let mut fallback: Option<(PathBuf, Vec<u8>)> = None;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| format!("Read installer entry: {err}"))?;
        let name = entry.name().to_string();
        if !name.ends_with("version.json") {
            continue;
        }
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Read installer version.json: {err}"))?;
        let path = PathBuf::from(name);
        fallback = Some((path, buffer));
        break;
    }

    if let Some((_, buffer)) = fallback {
        return Ok(buffer);
    }

    Err("NeoForge installer missing version.json".to_string().into())
}

fn read_profile(path: &Path) -> Result<VersionData, LauncherError> {
    let bytes =
        std::fs::read(path).map_err(|err| format!("Failed to read NeoForge profile: {err}"))?;
    serde_json::from_slice::<VersionData>(&bytes)
        .map_err(|err| format!("Failed to parse NeoForge profile: {err}").into())
}
