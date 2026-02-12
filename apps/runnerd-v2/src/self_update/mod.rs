use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use atlas_client::device_code::DEFAULT_ATLAS_HUB_URL;
use atlas_client::hub::{DistributionReleaseAsset, DistributionReleaseResponse, HubClient};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::time::{Duration, sleep};
use tracing::{debug, info, warn};

use crate::config;
use crate::supervisor::{SharedState, now_millis};

const UPDATE_INTERVAL_SECS: u64 = 6 * 60 * 60;
const SERVICE_PATH: &str = "/etc/systemd/system/atlas-runnerd.service";
const RUNNER_BIN_PATH: &str = "/usr/local/bin/atlas-runner";
const RUNNERD_BIN_FALLBACK_PATH: &str = "/usr/local/bin/atlas-runnerd";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StagedAsset {
    product: String,
    version: String,
    sha256: String,
    staged_path: String,
    checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StagedManifest {
    checked_at: String,
    assets: Vec<StagedAsset>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct InstalledVersions {
    #[serde(default)]
    runner: Option<String>,
    #[serde(default)]
    runnerd: Option<String>,
}

#[derive(Debug)]
struct ServiceReconcileResult {
    changed: bool,
    runnerd_exec_path: PathBuf,
}

pub fn start_background_update_loop(server_root: PathBuf, state: SharedState) {
    if let Err(reason) = activation_reason() {
        info!("runnerd self-update disabled: {reason}");
        let state_for_skip = state.clone();
        tokio::spawn(async move {
            set_update_error(&state_for_skip, reason).await;
        });
        return;
    }

    tokio::spawn(async move {
        if let Err(err) = check_and_stage_updates(&server_root, state.clone()).await {
            warn!("self-update startup check failed: {err}");
            set_update_error(&state, err).await;
        }

        loop {
            sleep(Duration::from_secs(UPDATE_INTERVAL_SECS)).await;
            if let Err(err) = check_and_stage_updates(&server_root, state.clone()).await {
                warn!("self-update periodic check failed: {err}");
                set_update_error(&state, err).await;
            }
        }
    });
}

pub async fn maybe_apply_staged_update(
    server_root: &PathBuf,
    state: SharedState,
) -> Result<(), String> {
    if let Err(reason) = activation_reason() {
        debug!("skipping self-update apply: {reason}");
        return Ok(());
    }

    let manifest_path = staged_manifest_path(server_root);
    if !manifest_path.exists() {
        return Ok(());
    }

    let manifest = read_staged_manifest(&manifest_path)?;
    if manifest.assets.is_empty() {
        return Ok(());
    }

    let service_path = PathBuf::from(SERVICE_PATH);
    let service_result = reconcile_service_file(&service_path)?;

    let mut installed = read_installed_versions(server_root);

    for asset in &manifest.assets {
        let target = match asset.product.as_str() {
            "runner" => PathBuf::from(RUNNER_BIN_PATH),
            "runnerd" => service_result.runnerd_exec_path.clone(),
            other => {
                warn!("unknown staged product '{other}', skipping");
                continue;
            }
        };

        let staged_path = PathBuf::from(&asset.staged_path);
        let bytes = fs::read(&staged_path).map_err(|err| {
            format!(
                "failed to read staged asset {}: {err}",
                staged_path.display()
            )
        })?;

        if !sha256_matches(&bytes, &asset.sha256) {
            return Err(format!(
                "staged asset hash mismatch for {}",
                staged_path.display()
            ));
        }

        write_binary_atomic(&target, &bytes)?;
        match asset.product.as_str() {
            "runner" => installed.runner = Some(normalize_version_for_compare(&asset.version)),
            "runnerd" => installed.runnerd = Some(normalize_version_for_compare(&asset.version)),
            _ => {}
        }
        info!(
            "applied staged {} update to {}",
            asset.product,
            target.display()
        );
    }

    write_installed_versions(server_root, &installed)?;

    if service_result.changed {
        run_systemctl(&["daemon-reload"])?;
        info!("reconciled atlas-runnerd.service managed keys");
    }

    cleanup_staged_files(server_root, &manifest);

    {
        let mut guard = state.lock().await;
        guard.self_update_last_applied_ms = Some(now_millis());
        guard.self_update_staged_version = None;
        guard.self_update_last_error = None;
    }

    info!("restarting atlas-runnerd.service to activate staged updates");
    run_systemctl(&["restart", "atlas-runnerd.service"])?;

    Ok(())
}

async fn check_and_stage_updates(server_root: &PathBuf, state: SharedState) -> Result<(), String> {
    let arch = normalize_distribution_arch(std::env::consts::ARCH)?;
    let mut hub = HubClient::new(&resolve_hub_url())
        .map_err(|err| format!("failed to create hub client for self-update: {err}"))?;
    if let Ok(token) = std::env::var("ATLAS_TOKEN") {
        if !token.trim().is_empty() {
            hub.set_token(token);
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let staged_dir = staged_assets_dir(server_root);
    fs::create_dir_all(&staged_dir).map_err(|err| {
        format!(
            "failed to create staged dir {}: {err}",
            staged_dir.display()
        )
    })?;

    let mut assets = Vec::new();

    let runnerd_current = normalize_version_for_compare(env!("ATLAS_BUILD_VERSION"));
    maybe_stage_product_update(
        &mut hub,
        "runnerd",
        arch,
        &runnerd_current,
        &staged_dir,
        &now,
        &mut assets,
    )
    .await?;

    let runner_current = current_runner_version(server_root);
    maybe_stage_product_update(
        &mut hub,
        "runner",
        arch,
        runner_current.as_deref().unwrap_or("0.0.0"),
        &staged_dir,
        &now,
        &mut assets,
    )
    .await?;

    if assets.is_empty() {
        remove_if_exists(&staged_manifest_path(server_root));
        {
            let mut guard = state.lock().await;
            guard.self_update_last_checked_ms = Some(now_millis());
            guard.self_update_last_error = None;
            guard.self_update_staged_version = None;
        }
        debug!("self-update check complete: no newer runner/runnerd releases");
        return Ok(());
    }

    let manifest = StagedManifest {
        checked_at: now,
        assets,
    };
    write_staged_manifest(server_root, &manifest)?;

    {
        let staged_summary = manifest
            .assets
            .iter()
            .map(|asset| format!("{}:{}", asset.product, asset.version))
            .collect::<Vec<_>>()
            .join(",");
        let mut guard = state.lock().await;
        guard.self_update_last_checked_ms = Some(now_millis());
        guard.self_update_last_error = None;
        guard.self_update_staged_version = Some(staged_summary);
    }

    info!(
        "staged runner self-update assets: {}",
        manifest.assets.len()
    );

    Ok(())
}

async fn maybe_stage_product_update(
    hub: &mut HubClient,
    product: &str,
    arch: &str,
    current_version: &str,
    staged_dir: &Path,
    checked_at: &str,
    staged_assets: &mut Vec<StagedAsset>,
) -> Result<(), String> {
    let release = hub
        .get_latest_distribution_release(product, "linux", arch)
        .await
        .map_err(|err| format!("failed to resolve latest {product} release: {err}"))?;

    if !is_outdated_version(current_version, &release.version) {
        debug!(
            "self-update: {product} already current (current={}, latest={})",
            current_version, release.version
        );
        return Ok(());
    }

    let asset = select_binary_asset(&release)?;
    let bytes = hub
        .download_distribution_asset(&asset.download_id)
        .await
        .map_err(|err| format!("failed to download {product} release asset: {err}"))?;

    if !sha256_matches(&bytes, &asset.sha256) {
        return Err(format!(
            "downloaded {product} asset hash mismatch for version {}",
            release.version
        ));
    }

    let staged_path = staged_dir.join(format!(
        "{}-{}",
        product,
        normalize_version_for_compare(&release.version)
    ));
    write_binary_atomic(&staged_path, &bytes)?;

    staged_assets.push(StagedAsset {
        product: product.to_string(),
        version: normalize_version_for_compare(&release.version),
        sha256: asset.sha256.clone(),
        staged_path: staged_path.to_string_lossy().to_string(),
        checked_at: checked_at.to_string(),
    });

    info!(
        "staged {} update: {} -> {}",
        product,
        normalize_version_for_compare(current_version),
        normalize_version_for_compare(&release.version)
    );

    Ok(())
}

fn set_non_root_reason() -> String {
    "process is not running as root; auto-update is disabled".to_string()
}

fn activation_reason() -> Result<(), String> {
    let managed_flag = std::env::var("ATLAS_SYSTEMD_MANAGED")
        .map(|value| value.trim() == "1")
        .unwrap_or(false);
    let uid = unsafe { libc::geteuid() as u32 };
    evaluate_activation(std::env::consts::OS, managed_flag, uid)
}

fn evaluate_activation(os: &str, systemd_managed: bool, uid: u32) -> Result<(), String> {
    if os != "linux" {
        return Err("self-update only runs on Linux".to_string());
    }
    if !systemd_managed {
        return Err(
            "ATLAS_SYSTEMD_MANAGED=1 was not detected; self-update requires systemd-managed mode"
                .to_string(),
        );
    }
    if uid != 0 {
        return Err(set_non_root_reason());
    }
    Ok(())
}

fn normalize_distribution_arch(arch: &str) -> Result<&'static str, String> {
    match arch {
        "x86_64" | "amd64" => Ok("x64"),
        "aarch64" | "arm64" => Ok("arm64"),
        other => Err(format!(
            "unsupported architecture '{other}' for self-update"
        )),
    }
}

fn resolve_hub_url() -> String {
    if let Ok(Some(config)) = config::load_deploy_key() {
        let trimmed = config.hub_url.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }

    if let Ok(value) = std::env::var("ATLAS_HUB_URL") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }

    DEFAULT_ATLAS_HUB_URL.to_string()
}

fn current_runner_version(server_root: &PathBuf) -> Option<String> {
    let output = Command::new(RUNNER_BIN_PATH).arg("--version").output().ok();
    if let Some(output) = output {
        let mut text = String::new();
        text.push_str(&String::from_utf8_lossy(&output.stdout));
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        if let Some(value) = extract_semver_token(&text) {
            return Some(normalize_version_for_compare(&value));
        }
    }

    let versions = read_installed_versions(server_root);
    versions
        .runner
        .map(|value| normalize_version_for_compare(&value))
}

fn extract_semver_token(value: &str) -> Option<String> {
    for token in value.split_whitespace() {
        let cleaned = token
            .trim_matches(|c: char| {
                !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
            })
            .trim_start_matches('v');
        if Version::parse(cleaned).is_ok() {
            return Some(cleaned.to_string());
        }
    }
    None
}

fn normalize_version_for_compare(value: &str) -> String {
    value.trim().trim_start_matches('v').to_string()
}

fn is_outdated_version(current: &str, latest: &str) -> bool {
    let current_norm = normalize_version_for_compare(current);
    let latest_norm = normalize_version_for_compare(latest);
    match (Version::parse(&current_norm), Version::parse(&latest_norm)) {
        (Ok(current_semver), Ok(latest_semver)) => current_semver < latest_semver,
        _ => current_norm != latest_norm,
    }
}

fn select_binary_asset(
    release: &DistributionReleaseResponse,
) -> Result<&DistributionReleaseAsset, String> {
    release
        .assets
        .iter()
        .find(|asset| asset.kind == "binary")
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.kind == "installer")
        })
        .ok_or_else(|| format!("no binary/installer asset found for {}", release.product))
}

fn sha256_matches(bytes: &[u8], expected_hex: &str) -> bool {
    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    actual.eq_ignore_ascii_case(expected_hex.trim())
}

fn updater_root(server_root: &PathBuf) -> PathBuf {
    server_root.join(".runner").join("self-update")
}

fn staged_assets_dir(server_root: &PathBuf) -> PathBuf {
    updater_root(server_root).join("staged")
}

fn staged_manifest_path(server_root: &PathBuf) -> PathBuf {
    updater_root(server_root).join("staged-manifest.json")
}

fn installed_versions_path(server_root: &PathBuf) -> PathBuf {
    updater_root(server_root).join("installed-versions.json")
}

fn read_staged_manifest(path: &Path) -> Result<StagedManifest, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("failed to read staged manifest {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| format!("failed to parse staged manifest {}: {err}", path.display()))
}

fn write_staged_manifest(server_root: &PathBuf, manifest: &StagedManifest) -> Result<(), String> {
    let path = staged_manifest_path(server_root);
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|err| format!("failed to serialize staged manifest: {err}"))?;
    write_text_atomic(&path, &content)
}

fn read_installed_versions(server_root: &PathBuf) -> InstalledVersions {
    let path = installed_versions_path(server_root);
    let content = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(_) => return InstalledVersions::default(),
    };

    serde_json::from_str::<InstalledVersions>(&content).unwrap_or_default()
}

fn write_installed_versions(
    server_root: &PathBuf,
    versions: &InstalledVersions,
) -> Result<(), String> {
    let path = installed_versions_path(server_root);
    let content = serde_json::to_string_pretty(versions)
        .map_err(|err| format!("failed to serialize installed versions marker: {err}"))?;
    write_text_atomic(&path, &content)
}

fn cleanup_staged_files(server_root: &PathBuf, manifest: &StagedManifest) {
    for asset in &manifest.assets {
        remove_if_exists(&PathBuf::from(&asset.staged_path));
    }
    remove_if_exists(&staged_manifest_path(server_root));
}

fn reconcile_service_file(path: &Path) -> Result<ServiceReconcileResult, String> {
    let original = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let (updated, changed, runnerd_exec_path) = reconcile_service_content(&original)?;
    if changed {
        write_text_atomic(path, &updated)?;
    }

    Ok(ServiceReconcileResult {
        changed,
        runnerd_exec_path,
    })
}

fn reconcile_service_content(content: &str) -> Result<(String, bool, PathBuf), String> {
    let lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
    let Some(service_start) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[Service]"))
    else {
        return Err("atlas-runnerd.service is missing [Service] section".to_string());
    };

    let service_end = lines
        .iter()
        .enumerate()
        .skip(service_start + 1)
        .find_map(|(idx, line)| {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                Some(idx)
            } else {
                None
            }
        })
        .unwrap_or(lines.len());

    let mut runnerd_exec_path: Option<PathBuf> = None;
    let mut filtered_service_lines = Vec::new();

    for line in &lines[service_start + 1..service_end] {
        if let Some((key, value)) = parse_key_value(line) {
            if key.eq_ignore_ascii_case("ExecStart") {
                if let Some(path) = parse_execstart_binary(value) {
                    runnerd_exec_path = Some(path);
                }
            }

            if is_managed_service_directive(key, value) {
                continue;
            }
        }

        filtered_service_lines.push(line.clone());
    }

    let managed_runnerd_path =
        runnerd_exec_path.unwrap_or_else(|| PathBuf::from(RUNNERD_BIN_FALLBACK_PATH));

    filtered_service_lines.push(format!("ExecStart={}", managed_runnerd_path.display()));
    filtered_service_lines.push("Restart=always".to_string());
    filtered_service_lines.push("RestartSec=5".to_string());
    filtered_service_lines.push("Environment=RUST_LOG=info".to_string());
    filtered_service_lines.push("Environment=ATLAS_SYSTEMD_MANAGED=1".to_string());

    let mut merged = Vec::new();
    merged.extend(lines[..service_start + 1].iter().cloned());
    merged.extend(filtered_service_lines);
    merged.extend(lines[service_end..].iter().cloned());

    let mut updated = merged.join("\n");
    if !updated.ends_with('\n') {
        updated.push('\n');
    }

    let mut normalized_original = content.to_string();
    if !normalized_original.ends_with('\n') {
        normalized_original.push('\n');
    }

    let changed = updated != normalized_original;
    Ok((updated, changed, managed_runnerd_path))
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn parse_execstart_binary(value: &str) -> Option<PathBuf> {
    let raw = value.trim().trim_start_matches('-');
    let first = raw.split_whitespace().next()?;
    if first.starts_with('/') {
        return Some(PathBuf::from(first));
    }
    None
}

fn is_managed_service_directive(key: &str, value: &str) -> bool {
    if key.eq_ignore_ascii_case("ExecStart")
        || key.eq_ignore_ascii_case("Restart")
        || key.eq_ignore_ascii_case("RestartSec")
    {
        return true;
    }

    if key.eq_ignore_ascii_case("Environment") {
        let normalized = value.trim().trim_matches('"').trim_matches('\'').trim();
        return normalized.starts_with("RUST_LOG=")
            || normalized.starts_with("ATLAS_SYSTEMD_MANAGED=");
    }

    false
}

fn remove_if_exists(path: &Path) {
    if !path.exists() {
        return;
    }

    if path.is_dir() {
        if let Err(err) = fs::remove_dir_all(path) {
            warn!("failed to remove {}: {}", path.display(), err);
        }
    } else if let Err(err) = fs::remove_file(path) {
        warn!("failed to remove {}: {}", path.display(), err);
    }
}

fn write_text_atomic(path: &Path, content: &str) -> Result<(), String> {
    let bytes = content.as_bytes();
    write_bytes_atomic(path, bytes, None)
}

fn write_binary_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_bytes_atomic(path, bytes, Some(0o755))
}

fn write_bytes_atomic(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| {
        format!(
            "path {} has no parent directory for atomic write",
            path.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;

    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, bytes)
        .map_err(|err| format!("failed to write {}: {err}", tmp_path.display()))?;

    if let Some(mode) = mode {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp_path, fs::Permissions::from_mode(mode)).map_err(|err| {
                format!("failed to set permissions on {}: {err}", tmp_path.display())
            })?;
        }
    }

    fs::rename(&tmp_path, path)
        .map_err(|err| format!("failed to move {} into place: {err}", path.display()))?;

    Ok(())
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|err| format!("failed to execute systemctl {:?}: {err}", args))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "systemctl {:?} failed (status={}): {}",
        args,
        output.status,
        stderr.trim()
    ))
}

async fn set_update_error(state: &SharedState, error: String) {
    let mut guard = state.lock().await;
    guard.self_update_last_error = Some(error);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_requires_linux_systemd_and_root() {
        assert!(evaluate_activation("linux", true, 0).is_ok());
        assert!(evaluate_activation("macos", true, 0).is_err());
        assert!(evaluate_activation("linux", false, 0).is_err());
        assert!(evaluate_activation("linux", true, 1000).is_err());
    }

    #[test]
    fn semver_extraction_finds_version_token() {
        let token = extract_semver_token("atlas-runner 1.2.3\n");
        assert_eq!(token.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn outdated_version_uses_semver_when_possible() {
        assert!(is_outdated_version("1.2.3", "1.2.4"));
        assert!(!is_outdated_version("1.2.3", "1.2.3"));
        assert!(!is_outdated_version("1.2.4", "1.2.3"));
    }

    #[test]
    fn sha256_verification_works() {
        let bytes = b"atlas";
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hex = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(sha256_matches(bytes, &hex));
        assert!(!sha256_matches(bytes, "deadbeef"));
    }

    #[test]
    fn service_reconcile_preserves_unknown_and_is_idempotent() {
        let original = "[Unit]\nDescription=Atlas Runner Daemon\n\n[Service]\nType=simple\nUser=atlas\nExecStart=/opt/bin/custom-runnerd --flag\nRestart=on-failure\nEnvironment=FOO=bar\n\n[Install]\nWantedBy=multi-user.target\n";

        let (first, changed, exec_path) = reconcile_service_content(original).unwrap();
        assert!(changed);
        assert_eq!(exec_path, PathBuf::from("/opt/bin/custom-runnerd"));
        assert!(first.contains("User=atlas"));
        assert!(first.contains("Environment=FOO=bar"));
        assert!(first.contains("ExecStart=/opt/bin/custom-runnerd"));
        assert!(first.contains("Environment=ATLAS_SYSTEMD_MANAGED=1"));

        let (second, changed_again, _) = reconcile_service_content(&first).unwrap();
        assert!(!changed_again);
        assert_eq!(first, second);
    }
}
