use crate::launcher::error::LauncherError;
use crate::net::http::shared_client;
use crate::paths::ensure_dir;
use futures::stream::{self, StreamExt};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::download::{
    download_if_needed_with_retry_events, DownloadRetryEvent, DOWNLOAD_CONCURRENCY,
};
use super::emit;
use super::libraries::current_arch;
use super::manifest::VersionData;
use crate::net::http::fetch_json;

const JAVA_RUNTIME_MANIFEST_URL: &str =
  "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const RUNTIME_MANIFEST_MARKER_FILE: &str = "runtime_manifest_url.txt";

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

#[derive(Debug, Deserialize)]
struct JavaRuntimeCatalogEntry {
    #[serde(default)]
    manifest: Option<JavaRuntimeCatalogManifest>,
    #[serde(default)]
    version: Option<JavaRuntimeCatalogVersion>,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeCatalogManifest {
    url: String,
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeCatalogVersion {
    #[serde(default)]
    name: Option<String>,
}

pub async fn resolve_java_path(
    window: &tauri::Window,
    game_dir: &Path,
    version_data: &VersionData,
    java_path_override: &str,
) -> Result<String, LauncherError> {
    let required_major = version_data
        .java_version
        .as_ref()
        .map(|java| java.major_version);

    if !java_path_override.trim().is_empty() && java_path_override.trim() != "java" {
        let override_path = normalize_java_override_path(java_path_override)?;
        validate_java_override(&override_path, required_major)?;
        return Ok(override_path);
    }
    let component = version_data
        .java_version
        .as_ref()
        .map(|java| java.component.clone())
        .unwrap_or_else(|| "jre-legacy".to_string());

    ensure_java_runtime(window, game_dir, &component, required_major).await
}

async fn ensure_java_runtime(
    window: &tauri::Window,
    game_dir: &Path,
    component: &str,
    required_major: Option<u32>,
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

    let entry = select_runtime_entry(platform, &chosen_component)?;
    let manifest_url = entry
        .manifest
        .as_ref()
        .map(|manifest| manifest.url.trim())
        .filter(|url| !url.is_empty())
        .ok_or_else(|| format!("Java runtime manifest url missing for {chosen_component}"))?;
    let runtime_manifest: JavaRuntimeFiles = fetch_json(&client, manifest_url).await?;

    let runtime_id = runtime_identifier(
        entry
            .version
            .as_ref()
            .and_then(|version| version.name.as_deref()),
        manifest_url,
    );
    let runtime_base = resolve_runtimes_root(game_dir)
        .join(os_key)
        .join(&chosen_component)
        .join(runtime_id);
    let runtime_home = runtime_base.join(&chosen_component);
    ensure_dir(&runtime_home)?;
    let marker_path = runtime_base.join(RUNTIME_MANIFEST_MARKER_FILE);

    let java_path = locate_java_binary(&runtime_home, &runtime_manifest);
    if runtime_is_latest(&java_path, &marker_path, manifest_url) {
        if let Err(err) = validate_runtime_install(&runtime_home, &runtime_manifest) {
            emit(
                window,
                "java",
                format!("Installed Java runtime failed validation; reinstalling ({err})"),
                None,
                None,
            )?;
        } else {
            emit(
                window,
                "java",
                format!("Using latest Java runtime ({chosen_component})"),
                None,
                None,
            )?;
            ensure_java_major_version(&java_path, required_major)?;
            return Ok(java_path.to_string_lossy().to_string());
        }
    }

    emit(
        window,
        "java",
        format!("Downloading Java runtime ({chosen_component})"),
        None,
        None,
    )?;

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
            let relative = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("runtime file")
                .to_string();
            async move {
                download_if_needed_with_retry_events(
                    &client,
                    &download,
                    &path,
                    |event: DownloadRetryEvent| {
                        let _ = emit(
                            window,
                            "java",
                            format!(
                                "Retrying Java runtime download ({relative}) {}/{} in {} ms ({})",
                                event.attempt, event.max_attempts, event.delay_ms, event.reason
                            ),
                            None,
                            None,
                        );
                    },
                )
                .await?;
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
    validate_runtime_install(&runtime_home, &runtime_manifest)
        .map_err(|err| format!("Java runtime validation failed after install: {err}"))?;
    ensure_java_major_version(&java_path, required_major)?;
    fs::write(&marker_path, manifest_url)
        .map_err(|err| format!("Failed to write runtime marker: {err}"))?;

    Ok(java_path.to_string_lossy().to_string())
}

fn validate_java_override(path: &str, required_major: Option<u32>) -> Result<(), String> {
    let candidate = Path::new(path);
    let looks_like_path = candidate.is_absolute() || path.contains('/') || path.contains('\\');

    if looks_like_path {
        if !candidate.exists() {
            return Err(format!("Configured Java path does not exist: {path}"));
        }
        if !candidate.is_file() {
            return Err(format!("Configured Java path is not a file: {path}"));
        }
    }

    if required_major.is_none() {
        return Ok(());
    }

    let detected_major = detect_java_major_version(path)?;
    if let Some(required) = required_major {
        if detected_major < required {
            return Err(format!(
                "Configured Java runtime is too old: detected Java {detected_major}, required Java {required}."
            ));
        }
    }

    Ok(())
}

pub(crate) fn normalize_java_override_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Configured Java path is empty.".to_string());
    }

    let candidate = Path::new(trimmed);
    let looks_like_path =
        candidate.is_absolute() || trimmed.contains('/') || trimmed.contains('\\');
    if !looks_like_path {
        return Ok(trimmed.to_string());
    }

    if !candidate.exists() {
        return Err(format!("Configured Java path does not exist: {trimmed}"));
    }
    if !candidate.is_file() {
        return Err(format!("Configured Java path is not a file: {trimmed}"));
    }
    if !is_executable_binary(candidate) {
        return Err(format!(
            "Configured Java path is not executable: {}",
            candidate.display()
        ));
    }

    let canonical = fs::canonicalize(candidate)
        .map_err(|err| format!("Failed to canonicalize Java path `{trimmed}`: {err}"))?;
    Ok(canonical.to_string_lossy().to_string())
}

fn is_executable_binary(path: &Path) -> bool {
    #[cfg(unix)]
    {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            // Some Windows Java distributions or wrappers may be extensionless.
            return true;
        };
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "exe" | "cmd" | "bat" | "com"
        )
    }
}

fn ensure_java_major_version(path: &Path, required_major: Option<u32>) -> Result<(), String> {
    let Some(required) = required_major else {
        return Ok(());
    };
    let binary = path.to_string_lossy();
    let detected = detect_java_major_version(&binary)?;
    if detected < required {
        return Err(format!(
            "Java runtime version mismatch: detected Java {detected}, required Java {required}."
        ));
    }
    Ok(())
}

fn detect_java_major_version(java_binary: &str) -> Result<u32, String> {
    let mut attempts = vec![java_binary.to_string()];
    if let Some(fallback) = java_version_fallback_binary(java_binary) {
        if !attempts.contains(&fallback) {
            attempts.push(fallback);
        }
    }

    let mut last_error = String::new();
    for candidate in attempts {
        let output = match Command::new(&candidate).arg("-version").output() {
            Ok(output) => output,
            Err(err) => {
                last_error = format!("Failed to run `{candidate} -version`: {err}");
                continue;
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            last_error = format!("`{candidate} -version` failed: {}", stderr.trim());
            continue;
        }

        let mut full = String::new();
        full.push_str(&String::from_utf8_lossy(&output.stdout));
        if !full.is_empty() && !output.stderr.is_empty() {
            full.push('\n');
        }
        full.push_str(&String::from_utf8_lossy(&output.stderr));

        if let Some(version) = parse_java_major_version(&full) {
            return Ok(version);
        }

        last_error =
            format!("Unable to parse Java major version from `{candidate} -version` output.");
    }

    if last_error.is_empty() {
        last_error = format!("Unable to parse Java major version for `{java_binary}`.");
    }
    Err(last_error)
}

pub(crate) fn parse_java_major_version(output: &str) -> Option<u32> {
    for line in output.lines() {
        if !line.to_ascii_lowercase().contains("version") {
            continue;
        }
        let Some(start) = line.find('"') else {
            continue;
        };
        let rest = &line[start + 1..];
        let Some(end) = rest.find('"') else {
            continue;
        };
        let quoted = &rest[..end];
        if let Some(major) = parse_java_major_from_version_token(quoted) {
            return Some(major);
        }
    }

    None
}

fn parse_java_major_from_version_token(version: &str) -> Option<u32> {
    let mut nums = version
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok());

    let first = nums.next()?;
    if first == 1 {
        nums.next()
    } else {
        Some(first)
    }
}

fn java_version_fallback_binary(java_binary: &str) -> Option<String> {
    let binary_lower = java_binary.to_ascii_lowercase();
    if binary_lower == "javaw" || binary_lower == "javaw.exe" {
        return Some("java".to_string());
    }

    let path = Path::new(java_binary);
    let file_name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if file_name == "javaw.exe" {
        let sibling = path.with_file_name("java.exe");
        return Some(sibling.to_string_lossy().to_string());
    }
    if file_name == "javaw" {
        let sibling = path.with_file_name("java");
        return Some(sibling.to_string_lossy().to_string());
    }

    None
}

pub(crate) fn is_java_ready_for_launch(game_dir: Option<&Path>, java_path_override: &str) -> bool {
    let trimmed = java_path_override.trim();
    if !trimmed.is_empty() {
        if trimmed.eq_ignore_ascii_case("java") {
            return java_on_path_exists();
        }
        if let Ok(normalized) = normalize_java_override_path(trimmed) {
            return validate_java_override(&normalized, None).is_ok();
        }
        return false;
    }

    if java_on_path_exists() {
        return true;
    }

    let Some(game_dir) = game_dir else {
        return false;
    };
    find_runtime_java_binary(&resolve_runtimes_root(game_dir))
        .map(|candidate| is_usable_java_binary(&candidate))
        .unwrap_or(false)
}

fn resolve_runtimes_root(game_dir: &Path) -> PathBuf {
    if let Some(instances_dir) = game_dir
        .ancestors()
        .find(|path| path.file_name() == Some(OsStr::new("instances")))
    {
        if let Some(root) = instances_dir.parent() {
            return root.join("runtimes");
        }
    }
    game_dir.join("runtimes")
}

fn java_on_path_exists() -> bool {
    let path_value = std::env::var_os("PATH");
    let Some(path_value) = path_value else {
        return false;
    };
    #[cfg(target_os = "windows")]
    let executable_names = ["java.exe", "javaw.exe", "java.cmd", "java.bat", "java"];
    #[cfg(not(target_os = "windows"))]
    let executable_names = ["java"];

    std::env::split_paths(&path_value)
        .flat_map(|dir| executable_names.iter().map(move |name| dir.join(name)))
        .any(|candidate| is_usable_java_binary(&candidate))
}

fn find_runtime_java_binary(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 6 || !dir.exists() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push((path, depth + 1));
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let name = name.to_ascii_lowercase();
            if name == "java" || name == "java.exe" || name == "javaw.exe" {
                return Some(path);
            }
        }
    }
    None
}

fn is_usable_java_binary(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(path) {
            return metadata.permissions().mode() & 0o111 != 0;
        }
        false
    }

    #[cfg(not(unix))]
    {
        let Some(ext) = path.extension().and_then(|value| value.to_str()) else {
            // Keep Windows readiness compatible with extensionless java wrappers.
            return true;
        };
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "exe" | "cmd" | "bat" | "com"
        )
    }
}

fn runtime_is_latest(java_path: &Path, marker_path: &Path, manifest_url: &str) -> bool {
    if !java_path.exists() {
        return false;
    }
    let marker = match fs::read_to_string(marker_path) {
        Ok(value) => value,
        Err(_) => return false,
    };
    marker.trim() == manifest_url.trim()
}

pub(crate) fn validate_runtime_install(
    runtime_home: &Path,
    manifest: &JavaRuntimeFiles,
) -> Result<(), String> {
    for (relative_path, entry) in manifest.files.iter() {
        let path = runtime_home.join(relative_path);
        match entry.kind.as_str() {
            "directory" => {
                if !path.is_dir() {
                    return Err(format!("Missing runtime directory: {}", path.display()));
                }
            }
            "file" => {
                if !path.is_file() {
                    return Err(format!("Missing runtime file: {}", path.display()));
                }

                if let Some(download) = entry.downloads.as_ref().and_then(|d| d.raw.as_ref()) {
                    if let Some(expected_size) = download.size {
                        let actual_size = fs::metadata(&path)
                            .map_err(|err| format!("Failed to stat {}: {err}", path.display()))?
                            .len();
                        if actual_size != expected_size {
                            return Err(format!(
                                "Size mismatch for {}: expected {}, got {}",
                                path.display(),
                                expected_size,
                                actual_size
                            ));
                        }
                    }

                    if let Some(expected_sha1) = download.sha1.as_deref() {
                        let actual_sha1 = sha1_file(&path)?;
                        if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
                            return Err(format!(
                                "SHA-1 mismatch for {}: expected {}, got {}",
                                path.display(),
                                expected_sha1,
                                actual_sha1
                            ));
                        }
                    }
                }
            }
            "link" => {
                if !path.exists() {
                    return Err(format!("Missing runtime link: {}", path.display()));
                }
            }
            _ => {}
        }
    }
    Ok(())
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

fn select_runtime_entry(
    platform: &serde_json::Map<String, serde_json::Value>,
    component: &str,
) -> Result<JavaRuntimeCatalogEntry, String> {
    let entries = platform
        .get(component)
        .and_then(|value| value.as_array())
        .ok_or_else(|| "No Java runtime components available for this platform.".to_string())?;

    for value in entries {
        let Ok(entry) = serde_json::from_value::<JavaRuntimeCatalogEntry>(value.clone()) else {
            continue;
        };
        if entry
            .manifest
            .as_ref()
            .map(|manifest| !manifest.url.trim().is_empty())
            .unwrap_or(false)
        {
            return Ok(entry);
        }
    }

    Err("No Java runtime entries available for this platform.".to_string())
}

fn runtime_identifier(version_name: Option<&str>, manifest_url: &str) -> String {
    let mut sanitized = String::new();
    for ch in version_name
        .unwrap_or("runtime")
        .trim()
        .to_ascii_lowercase()
        .chars()
    {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
            continue;
        }
        if ch == '.' || ch == '_' || ch == '-' {
            sanitized.push('-');
        }
    }
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        sanitized = "runtime".to_string();
    }

    let mut hasher = Sha1::new();
    hasher.update(manifest_url.as_bytes());
    let digest = hex::encode(hasher.finalize());
    let hash = &digest[..12];
    format!("{sanitized}-{hash}")
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

fn sha1_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|err| format!("Failed to open {}: {err}", path.display()))?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
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
