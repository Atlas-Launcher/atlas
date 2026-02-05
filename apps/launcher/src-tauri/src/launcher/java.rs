use crate::launcher::error::LauncherError;
use crate::net::http::shared_client;
use crate::paths::ensure_dir;
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::download::{download_if_needed, DOWNLOAD_CONCURRENCY};
use super::emit;
use super::libraries::current_arch;
use super::manifest::VersionData;
use crate::net::http::fetch_json;

const JAVA_RUNTIME_MANIFEST_URL: &str =
  "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Deserialize)]
pub(crate) struct JavaRuntimeFiles {
    pub files: HashMap<String, JavaRuntimeFile>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JavaRuntimeFile {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub downloads: Option<JavaRuntimeDownloads>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JavaRuntimeDownloads {
    #[serde(default)]
    pub raw: Option<super::manifest::Download>,
}

pub async fn resolve_java_path(
    window: &tauri::Window,
    game_dir: &Path,
    version_data: &VersionData,
    java_path_override: &str,
) -> Result<String, LauncherError> {
    if !java_path_override.trim().is_empty() && java_path_override.trim() != "java" {
        return Ok(java_path_override.trim().to_string());
    }
    let component = version_data
        .java_version
        .as_ref()
        .map(|java| java.component.clone())
        .unwrap_or_else(|| "jre-legacy".to_string());

    ensure_java_runtime(window, game_dir, &component).await
}

async fn ensure_java_runtime(
    window: &tauri::Window,
    game_dir: &Path,
    component: &str,
) -> Result<String, LauncherError> {
    let client = shared_client().clone();
    let os_key = runtime_os_key()?;

    emit(
        window,
        "java",
        format!("Checking Java runtime ({component})"),
        None,
        None,
    )?;

    let manifest: serde_json::Value = fetch_json(&client, JAVA_RUNTIME_MANIFEST_URL).await?;
    let platform = manifest
        .get(os_key)
        .and_then(|value| value.as_object())
        .ok_or_else(|| format!("Java runtime platform {os_key} not found"))?;

    let chosen_component = select_java_component(platform, component);
    if chosen_component != component {
        emit(
            window,
            "java",
            format!("Java runtime {component} not found. Using {chosen_component} instead."),
            None,
            None,
        )?;
    }

    let entry_list = platform
        .get(&chosen_component)
        .and_then(|value| value.as_array())
        .ok_or_else(|| "No Java runtime components available for this platform.".to_string())?;
    let entry = entry_list
        .iter()
        .find_map(|value| value.as_object())
        .ok_or_else(|| "No Java runtime entries available for this platform.".to_string())?;
    let manifest_url = entry
        .get("manifest")
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| format!("Java runtime manifest url missing for {chosen_component}"))?;

    emit(
        window,
        "java",
        format!("Downloading Java runtime ({component})"),
        None,
        None,
    )?;

    let runtime_manifest: JavaRuntimeFiles = fetch_json(&client, manifest_url).await?;
    let runtime_base = game_dir.join("runtime").join(component).join(os_key);
    let runtime_home = runtime_base.join(component);
    ensure_dir(&runtime_home)?;

    let mut downloads: Vec<(super::manifest::Download, PathBuf, bool)> = Vec::new();
    let mut links: Vec<(PathBuf, PathBuf)> = Vec::new();

    for (relative_path, file) in runtime_manifest.files.iter() {
        let out_path = runtime_home.join(relative_path);

        match file.kind.as_str() {
            "directory" => {
                ensure_dir(&out_path)?;
            }
            "file" => {
                let download = file
                    .downloads
                    .as_ref()
                    .and_then(|d| d.raw.as_ref())
                    .ok_or_else(|| {
                        format!("Missing raw download for Java runtime file {relative_path}")
                    })?;
                downloads.push((download.clone(), out_path, file.executable));
            }
            "link" => {
                if let Some(target) = &file.target {
                    let base = out_path.parent().unwrap_or(&runtime_home);
                    let target_path = base.join(target);
                    links.push((target_path, out_path));
                }
            }
            _ => {}
        }
    }

    let total = downloads.len() as u64;
    let mut index = 0u64;
    if total > 0 {
        let mut stream = stream::iter(downloads.into_iter().map(|(download, path, executable)| {
            let client = client.clone();
            async move {
                download_if_needed(&client, &download, &path).await?;
                if executable {
                    set_executable(&path)?;
                }
                Ok::<(), String>(())
            }
        }))
        .buffer_unordered(DOWNLOAD_CONCURRENCY);

        while let Some(result) = stream.next().await {
            result?;
            index += 1;
            if index % 200 == 0 || index == total {
                emit(
                    window,
                    "java",
                    format!("Java runtime files {index}/{total}"),
                    Some(index),
                    Some(total),
                )?;
            }
        }
    }

    for (target, link) in links {
        create_runtime_link(&target, &link)?;
    }

    let java_path = locate_java_binary(&runtime_home, &runtime_manifest);
    if !java_path.exists() {
        return Err(
            "Java runtime download completed but java binary was not found."
                .to_string()
                .into(),
        );
    }

    Ok(java_path.to_string_lossy().to_string())
}

fn runtime_os_key() -> Result<&'static str, String> {
    if cfg!(target_os = "windows") {
        return Ok(match current_arch() {
            "64" => "windows-x64",
            "32" => "windows-x86",
            "arm64" => "windows-arm64",
            _ => "windows-x64",
        });
    }
    if cfg!(target_os = "macos") {
        return Ok(match current_arch() {
            "arm64" => "mac-os-arm64",
            _ => "mac-os",
        });
    }
    if cfg!(target_os = "linux") {
        return Ok(match current_arch() {
            "32" => "linux-i386",
            "arm64" => "linux-arm64",
            _ => "linux",
        });
    }
    Err("Unsupported OS for Java runtime downloads.".to_string())
}

pub(crate) fn select_java_component(
    platform: &serde_json::Map<String, serde_json::Value>,
    desired: &str,
) -> String {
    if platform
        .get(desired)
        .and_then(|value| value.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false)
    {
        return desired.to_string();
    }

    let mut candidates = vec![
        "java-runtime-delta",
        "java-runtime-gamma",
        "java-runtime-beta",
        "java-runtime-alpha",
        "jre-legacy",
    ];

    if !candidates.iter().any(|item| *item == desired) {
        candidates.insert(0, desired);
    }

    for candidate in candidates {
        if platform
            .get(candidate)
            .and_then(|value| value.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false)
        {
            return candidate.to_string();
        }
    }

    platform
        .keys()
        .next()
        .cloned()
        .unwrap_or_else(|| desired.to_string())
}

pub(crate) fn locate_java_binary(runtime_home: &Path, manifest: &JavaRuntimeFiles) -> PathBuf {
    if cfg!(target_os = "windows") {
        let javaw = runtime_home.join("bin").join("javaw.exe");
        if javaw.exists() {
            return javaw;
        }
        let java = runtime_home.join("bin").join("java.exe");
        if java.exists() {
            return java;
        }
    } else {
        let java = runtime_home.join("bin").join("java");
        if java.exists() {
            return java;
        }
    }

    for (relative_path, file) in manifest.files.iter() {
        if file.kind != "file" || !file.executable {
            continue;
        }
        let lower = relative_path.to_lowercase();
        if lower.ends_with("/bin/java") || lower.ends_with("\\bin\\java") {
            return runtime_home.join(relative_path);
        }
        if cfg!(target_os = "windows") {
            if lower.ends_with("/bin/java.exe") || lower.ends_with("\\bin\\java.exe") {
                return runtime_home.join(relative_path);
            }
            if lower.ends_with("/bin/javaw.exe") || lower.ends_with("\\bin\\javaw.exe") {
                return runtime_home.join(relative_path);
            }
        }
    }

    runtime_home.join("bin").join("java")
}

fn set_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|err| format!("Failed to read permissions: {err}"))?
            .permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path, perms)
            .map_err(|err| format!("Failed to set executable permission: {err}"))?;
    }
    Ok(())
}

fn create_runtime_link(target: &Path, link: &Path) -> Result<(), String> {
    if link.exists() {
        return Ok(());
    }
    if let Some(parent) = link.parent() {
        ensure_dir(parent)?;
    }
    if !target.exists() {
        return Err(format!(
            "Java runtime link target missing: {}",
            target.display()
        ));
    }

    if try_create_symlink(target, link).is_ok() {
        return Ok(());
    }

    if target.is_file() {
        std::fs::copy(target, link)
            .map_err(|err| format!("Failed to copy Java runtime link: {err}"))?;
        return Ok(());
    }

    if target.is_dir() {
        ensure_dir(link)?;
    }
    Ok(())
}

fn try_create_symlink(target: &Path, link: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
            .map_err(|err| format!("Failed to create symlink: {err}"))?;
        return Ok(());
    }
    #[cfg(windows)]
    {
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)
                .map_err(|err| format!("Failed to create symlink: {err}"))?;
        } else {
            std::os::windows::fs::symlink_file(target, link)
                .map_err(|err| format!("Failed to create symlink: {err}"))?;
        }
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err("Symlinks are not supported on this platform.".to_string())
}
