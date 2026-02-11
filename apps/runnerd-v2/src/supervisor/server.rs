use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlas_client::hub::HubClient;
use runner_core_v2::proto::*;
use runner_provision_v2::{ensure_applied_from_packblob_bytes, DependencyProvider, LaunchPlan};
use runner_v2_rcon::{load_rcon_settings, RconClient};
use sysinfo::System;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use super::logs::LogStore;
use super::monitor::ensure_monitor;
use super::state::{ServerState, SharedState};
use super::updates::{ensure_watchers, sync_whitelist_to_root};
use super::util::{default_server_root, now_millis};
use crate::config;

fn get_default_max_ram_mb() -> u32 {
    let mut system = System::new();
    system.refresh_memory();
    let total_gb = (system.total_memory() / 1024 / 1024 / 1024) as u32;
    let target_gb = if total_gb <= 8 {
        (total_gb.saturating_sub(2)).max(1)
    } else {
        #[cfg(target_os = "macos")]
        {
            8
        }
        #[cfg(target_os = "linux")]
        {
            if total_gb == 16 {
                14
            } else if total_gb >= 24 {
                16
            } else {
                8
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            8
        }
    };
    target_gb * 1024
}

fn normalize_max_ram_mb(value: u32) -> u32 {
    if value == 0 {
        return get_default_max_ram_mb();
    }
    // Backward compatibility: older config values may be GB-like.
    if value <= 64 {
        return value * 1024;
    }
    value
}

fn is_java_command_token(value: &str) -> bool {
    let token = value.rsplit(['/', '\\']).next().unwrap_or(value);
    token.eq_ignore_ascii_case("java")
        || token.eq_ignore_ascii_case("java.exe")
        || token.eq_ignore_ascii_case("javaw.exe")
}

fn ensure_memory_flags(argv: &mut Vec<String>, max_ram_mb: u32) -> bool {
    if !argv.iter().any(|arg| is_java_command_token(arg)) {
        return false;
    }

    argv.retain(|arg| {
        let lower = arg.to_ascii_lowercase();
        !(lower.starts_with("-xmx") || lower.starts_with("-xms"))
    });

    let Some(java_pos) = argv.iter().position(|arg| is_java_command_token(arg)) else {
        return false;
    };
    let insert_pos = java_pos + 1;

    argv.insert(insert_pos, format!("-Xms{}m", max_ram_mb));
    argv.insert(insert_pos + 1, format!("-Xmx{}m", max_ram_mb));
    true
}

#[cfg(test)]
mod tests {
    use super::{ensure_memory_flags, is_java_command_token, normalize_max_ram_mb};

    #[test]
    fn java_token_detection_accepts_absolute_paths() {
        assert!(is_java_command_token("java"));
        assert!(is_java_command_token("/opt/jdk/bin/java"));
        assert!(is_java_command_token("C:\\Java\\bin\\java.exe"));
        assert!(is_java_command_token("C:\\Java\\bin\\javaw.exe"));
        assert!(!is_java_command_token("python"));
    }

    #[test]
    fn memory_flags_insert_after_java_path_and_replace_existing() {
        let mut argv = vec![
            "/opt/jdk/bin/java".to_string(),
            "-Xmx512m".to_string(),
            "-jar".to_string(),
            "server.jar".to_string(),
        ];
        ensure_memory_flags(&mut argv, 4096);
        assert_eq!(argv[0], "/opt/jdk/bin/java");
        assert_eq!(argv[1], "-Xms4096m");
        assert_eq!(argv[2], "-Xmx4096m");
        assert_eq!(argv[3], "-jar");
    }

    #[test]
    fn normalize_max_ram_supports_legacy_gb_values() {
        assert_eq!(normalize_max_ram_mb(8), 8192);
        assert_eq!(normalize_max_ram_mb(4096), 4096);
    }

    #[test]
    fn memory_flags_not_inserted_for_non_java_commands() {
        let mut argv = vec!["python".to_string(), "server.py".to_string()];
        let changed = ensure_memory_flags(&mut argv, 4096);
        assert!(!changed);
        assert_eq!(argv, vec!["python".to_string(), "server.py".to_string()]);
    }
}

pub async fn start_server_from_deploy(state: SharedState) {
    {
        let mut guard = state.lock().await;
        if guard.is_running() {
            return;
        }
        // Ensure logs are initialized for auto-start
        if guard.logs.server_subscribe().is_empty() {
            guard.logs = super::logs::LogStore::new(2000);
        }
    }

    let deploy = match config::load_deploy_key() {
        Ok(Some(value)) => value,
        Ok(None) => {
            warn!("deploy key not configured; skipping auto-start");
            return;
        }
        Err(err) => {
            warn!("failed to load deploy key config: {err}");
            return;
        }
    };

    if !deploy.should_autostart.unwrap_or(false) {
        info!("auto-start disabled in config");
        return;
    }

    let mut hub = match HubClient::new(&deploy.hub_url) {
        Ok(value) => value,
        Err(err) => {
            warn!("failed to create hub client: {err}");
            return;
        }
    };
    hub.set_service_token(deploy.deploy_key.clone());
    let hub = Arc::new(hub);

    info!("auto-starting server from deploy key");
    let server_root = default_server_root("default");
    let current_build_id = load_current_build_id(&server_root).await;
    // reflect on-disk current build id in shared state
    if let Some(ref id) = current_build_id {
        let mut guard = state.lock().await;
        guard.current_pack_build_id = Some(id.clone());
    }

    let artifact = match hub
        .get_launcher_artifact(
            &deploy.pack_id,
            &deploy.channel,
            current_build_id.as_deref(),
        )
        .await
    {
        Ok(value) => value,
        Err(err) => {
            warn!("get artifact failed: {err}");
            return;
        }
    };

    let build = if let Some(ref current) = current_build_id {
        if artifact.build_id.as_ref() == Some(current) {
            info!("server is already up to date (build_id: {})", current);
            // Load stored pack_blob
            match load_pack_blob(&server_root).await {
                Some(blob) => blob,
                None => {
                    warn!("no stored pack blob, downloading");
                    match hub.download_blob(&artifact.download_url).await {
                        Ok(blob) => blob,
                        Err(err) => {
                            warn!("download build failed: {err}");
                            return;
                        }
                    }
                }
            }
        } else {
            match hub.download_blob(&artifact.download_url).await {
                Ok(blob) => blob,
                Err(err) => {
                    warn!("download build failed: {err}");
                    return;
                }
            }
        }
    } else {
        match hub.download_blob(&artifact.download_url).await {
            Ok(blob) => blob,
            Err(err) => {
                warn!("download build failed: {err}");
                return;
            }
        }
    };

    let profile = "default".to_string();

    if let Err(err) = start_server(profile, &build, server_root.clone(), state.clone()).await {
        warn!("failed to auto-start server: {}", err.message);
        return;
    }

    // If artifact requested a force reinstall or full reinstall, ensure existing current is cleared
    // before continuing. We move it to .runner/backup/current-<ms> to keep a backup.
    if artifact.force_reinstall.unwrap_or(false)
        || artifact.requires_full_reinstall.unwrap_or(false)
    {
        info!("artifact requests force reinstall; clearing existing server data before auto-start");
        let current = server_root.join("current");
        match tokio::fs::try_exists(&current).await {
            Ok(true) => {
                let backup_dir = server_root.join(".runner").join("backup");
                if let Err(err) = tokio::fs::create_dir_all(&backup_dir).await {
                    warn!(
                        "failed to create backup dir {}: {}",
                        backup_dir.display(),
                        err
                    );
                }
                let backup = backup_dir.join(format!("current-{}", super::util::now_millis()));
                match tokio::fs::rename(&current, &backup).await {
                    Ok(_) => info!("moved existing current to backup: {}", backup.display()),
                    Err(err) => {
                        warn!("failed to move current to backup: {}", err);
                        if let Err(err2) = tokio::fs::remove_dir_all(&current).await {
                            warn!(
                                "failed to remove existing current after rename failure: {}",
                                err2
                            );
                        } else {
                            info!("removed existing current directory");
                        }
                    }
                }
            }
            Ok(false) => {
                debug!("no existing current directory to clear");
            }
            Err(err) => {
                warn!("failed to probe existing current directory: {}", err);
            }
        }
    }

    info!("server auto-started successfully");

    // Save the new build_id and pack_blob. Update shared state immediately so the
    // in-memory state reflects the deployed build even if persisting to disk fails.
    if let Some(build_id) = artifact.build_id {
        // update shared state first
        {
            let mut guard = state.lock().await;
            guard.current_pack_build_id = Some(build_id.clone());
        }

        if let Err(err) = save_current_build_id(&server_root, &build_id).await {
            warn!("failed to save build_id: {err}");
        }

        if let Err(err) = save_pack_blob(&server_root, &build).await {
            warn!("failed to save pack_blob: {err}");
        }
    }
}

pub async fn build_status(
    daemon_start_ms: u64,
    state: &SharedState,
) -> (DaemonStatus, ServerStatus) {
    let mut guard = state.lock().await;
    refresh_child_status(&mut guard).await;

    let daemon = DaemonStatus {
        daemon_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: runner_core_v2::PROTOCOL_VERSION,
        pid: std::process::id() as i32,
        uptime_ms: now_millis().saturating_sub(daemon_start_ms),
    };

    (daemon, guard.status.clone())
}

pub async fn start_server(
    profile: ProfileId,
    pack_blob_bytes: &[u8],
    server_root: PathBuf,
    state: SharedState,
) -> Result<Response, RpcError> {
    // Quick check for already running. We will also acquire the lifecycle lock
    // to serialize start/stop/update operations.
    {
        let state_guard = state.lock().await;
        if state_guard.is_running() {
            return Err(RpcError {
                code: ErrorCode::ServerAlreadyRunning,
                message: "server already running".into(),
                details: Default::default(),
            });
        }
    }

    // Acquire lifecycle lock with timeout so concurrent lifecycle ops don't run together.
    let lifecycle_lock = {
        let guard = state.lock().await;
        guard.lifecycle_lock.clone()
    };
    match tokio::time::timeout(Duration::from_secs(5), lifecycle_lock.lock()).await {
        Ok(_l) => {
            // we hold lifecycle lock for the duration of start
        }
        Err(_) => {
            return Err(RpcError {
                code: ErrorCode::Internal,
                message: "another lifecycle operation in progress".into(),
                details: Default::default(),
            });
        }
    }

    let launch_plan = apply_pack_blob(&server_root, &pack_blob_bytes).await?;
    if let Ok(Some(deploy)) = config::load_deploy_key() {
        if let Ok(mut hub) = HubClient::new(&deploy.hub_url) {
            hub.set_service_token(deploy.deploy_key.clone());
            let hub = Arc::new(hub);
            if let Err(err) =
                sync_whitelist_to_root(hub, &deploy.pack_id, &server_root, state.clone()).await
            {
                warn!("whitelist sync failed on start: {err}");
            }
        }
    }
    let logs = {
        let guard = state.lock().await;
        guard.logs.clone()
    };
    let spawn_logs = logs.clone();
    let child = spawn_server(&launch_plan, &server_root, &BTreeMap::new(), spawn_logs)
        .await
        .map_err(|err| RpcError {
            code: ErrorCode::Internal,
            message: format!("failed to start server: {err}"),
            details: Default::default(),
        })?;

    let pid = child.id().unwrap_or_default() as i32;
    let started_at_ms = now_millis();

    {
        let mut guard = state.lock().await;
        guard.child = Some(child);
        guard.profile = Some(profile.clone());
        guard.server_root = Some(server_root.clone());
        guard.launch_plan = Some(launch_plan);
        guard.restart_attempts = 0;
        guard.restart_disabled = false;
        guard.last_start_ms = Some(started_at_ms);
        guard.status = ServerStatus::Running {
            profile: profile.clone(),
            pid,
            started_at_ms,
            meta: Default::default(),
        };
    }

    logs.push_daemon(format!(
        "server started: profile={} pid={} root={}",
        profile,
        pid,
        server_root.display()
    ));

    // Start monitor; update watchers are started by the daemon bootstrap path to avoid
    // mutually recursive async dependencies between modules.
    ensure_monitor(state.clone()).await;

    Ok(Response::Started {
        profile,
        pid,
        started_at_ms,
    })
}

pub async fn stop_server(force: bool, state: SharedState) -> Result<Response, RpcError> {
    // Acquire lifecycle lock to serialize stop with other lifecycle operations
    let lifecycle_lock = {
        let guard = state.lock().await;
        guard.lifecycle_lock.clone()
    };
    match tokio::time::timeout(Duration::from_secs(5), lifecycle_lock.lock()).await {
        Ok(_l) => {}
        Err(_) => {
            return Err(RpcError {
                code: ErrorCode::Internal,
                message: "another lifecycle operation in progress".into(),
                details: Default::default(),
            });
        }
    }

    // Request watcher worker to stop gracefully (if present)
    // We set the flag under the state lock, then drop before calling the internal stop
    // which also acquires the state lock internally.
    let watcher_done_opt = {
        let mut guard = state.lock().await;
        refresh_child_status(&mut guard).await;

        if let Some(ref flag) = guard.watcher_stop {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            // mark watchers as not started so future start will re-create them
            guard.watchers_started = false;
        }

        guard.watcher_done.clone()
    };

    // Stop the server (graceful or force) using the internal helper which will handle killing the child
    if let Err(err) = stop_server_internal(state.clone(), force).await {
        return Err(RpcError {
            code: err.code,
            message: format!("failed to stop server: {}", err.message),
            details: err.details,
        });
    }

    // Wait for watcher to signal done via the atomic flag (up to 10s).
    if let Some(done_flag) = watcher_done_opt {
        match tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if done_flag.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        })
        .await
        {
            Ok(_) => info!("watcher worker signaled done"),
            Err(_) => warn!("timed out waiting for watcher worker to signal done"),
        }
    }

    let mut guard = state.lock().await;
    let profile = guard.profile.clone().unwrap_or_else(|| "default".into());
    let stopped_at_ms = now_millis();
    let exit_info = ExitInfo {
        code: None,
        signal: None,
    };
    guard.child = None;
    guard.status = ServerStatus::Exited {
        profile: profile.clone(),
        exit: exit_info.clone(),
        at_ms: stopped_at_ms,
    };

    let logs = guard.logs.clone();
    logs.push_daemon(format!("server stopped: profile={profile}"));

    Ok(Response::Stopped {
        exit: Some(exit_info),
        stopped_at_ms,
    })
}

async fn refresh_child_status(state: &mut ServerState) {
    let Some(child) = state.child.as_mut() else {
        return;
    };

    match child.try_wait() {
        Ok(Some(status)) => {
            let exit = ExitInfo {
                code: status.code(),
                signal: None,
            };
            let profile = state.profile.clone().unwrap_or_else(|| "default".into());
            state.child = None;
            state.status = ServerStatus::Exited {
                profile,
                exit,
                at_ms: now_millis(),
            };
        }
        Ok(None) => {}
        Err(_) => {}
    }
}

pub(crate) async fn apply_pack_blob(
    server_root: &PathBuf,
    pack_blob: &[u8],
) -> Result<LaunchPlan, RpcError> {
    let provider = HttpDependencyProvider::default();
    let mut launch_plan = ensure_applied_from_packblob_bytes(server_root, pack_blob, &provider)
        .await
        .map_err(|err| RpcError {
            code: ErrorCode::InvalidConfig,
            message: format!("provision failed: {err}"),
            details: Default::default(),
        })?;

    // Add RAM limits if specified in config
    if let Ok(Some(deploy_config)) = config::load_deploy_key() {
        let max_ram_mb = deploy_config
            .max_ram
            .map(normalize_max_ram_mb)
            .unwrap_or_else(get_default_max_ram_mb);
        if ensure_memory_flags(&mut launch_plan.argv, max_ram_mb) {
            info!(
                "applied JVM memory flags: -Xms{}m -Xmx{}m",
                max_ram_mb, max_ram_mb
            );
        } else {
            warn!("launch plan command is not java; skipping JVM memory flags");
        }
    }

    Ok(launch_plan)
}

pub(crate) async fn stop_server_internal(state: SharedState, force: bool) -> Result<(), RpcError> {
    let server_root = super::util::current_server_root(&state)
        .await
        .ok_or_else(|| RpcError {
            code: ErrorCode::InvalidConfig,
            message: "server root not configured".into(),
            details: Default::default(),
        })?;

    if let Ok(Some(settings)) = load_rcon_settings(&server_root.join("current")).await {
        let rcon = RconClient::new(settings.address, settings.password);
        let _ = rcon.execute("stop").await;
    }

    let mut child = {
        let mut guard = state.lock().await;
        guard.child.take()
    };

    if let Some(ref mut child) = child {
        if !force {
            // Wait up to 30 seconds (60 * 500ms) for the server process to exit gracefully
            info!("attempting graceful shutdown, waiting up to 30 seconds for process to exit...");
            for _ in 0..60 {
                if let Ok(Some(_)) = child.try_wait() {
                    return Ok(());
                }
                sleep(Duration::from_millis(500)).await;
            }
        }

        child.kill().await.map_err(|err| RpcError {
            code: ErrorCode::IoError,
            message: format!("failed to kill server: {err}"),
            details: Default::default(),
        })?;
    }

    Ok(())
}

pub(crate) async fn spawn_server(
    plan: &LaunchPlan,
    server_root: &PathBuf,
    env: &BTreeMap<String, String>,
    logs: LogStore,
) -> Result<tokio::process::Child, std::io::Error> {
    let cwd = server_root.join("current").join(&plan.cwd_rel);
    let mut argv = plan.argv.iter();
    let program = argv.next().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty launch command")
    })?;

    let mut cmd = Command::new(program);
    cmd.args(argv);
    cmd.current_dir(&cwd);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    for (key, value) in env {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn()?;

    if let Some(stdout) = child.stdout.take() {
        let stdout_logs = logs.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                stdout_logs.push_server(LogStream::Stdout, line);
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let stderr_logs = logs.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                stderr_logs.push_server(LogStream::Stderr, line);
            }
        });
    }

    Ok(child)
}

struct HttpDependencyProvider {
    client: reqwest::Client,
}

impl Default for HttpDependencyProvider {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}
#[async_trait::async_trait]
impl DependencyProvider for HttpDependencyProvider {
    async fn fetch(
        &self,
        dep: &protocol::Dependency,
    ) -> Result<Vec<u8>, runner_provision_v2::errors::ProvisionError> {
        let response = self
            .client
            .get(&dep.url)
            .send()
            .await
            .map_err(|err| {
                runner_provision_v2::errors::ProvisionError::Invalid(format!(
                    "dependency download failed: {err}"
                ))
            })?
            .error_for_status()
            .map_err(|err| {
                runner_provision_v2::errors::ProvisionError::Invalid(format!(
                    "dependency download failed: {err}"
                ))
            })?;
        let bytes = response.bytes().await.map_err(|err| {
            runner_provision_v2::errors::ProvisionError::Invalid(format!(
                "dependency download failed: {err}"
            ))
        })?;
        Ok(bytes.to_vec())
    }
}

async fn load_current_build_id(server_root: &PathBuf) -> Option<String> {
    let path = server_root
        .join("current")
        .join(".runner")
        .join("build_id.txt");
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Some(content.trim().to_string()),
        Err(_) => None,
    }
}

async fn save_current_build_id(server_root: &PathBuf, build_id: &str) -> std::io::Result<()> {
    let dir = server_root.join("current").join(".runner");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("build_id.txt");
    tokio::fs::write(path, build_id).await
}

async fn load_pack_blob(server_root: &PathBuf) -> Option<Vec<u8>> {
    let path = server_root
        .join("current")
        .join(".runner")
        .join("pack_blob.bin");
    match tokio::fs::read(&path).await {
        Ok(blob) => Some(blob),
        Err(_) => None,
    }
}

async fn save_pack_blob(server_root: &PathBuf, blob: &[u8]) -> std::io::Result<()> {
    let dir = server_root.join("current").join(".runner");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("pack_blob.bin");
    tokio::fs::write(path, blob).await
}
