use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use atlas_client::hub::HubClient;
use runner_core_v2::proto::*;
use runner_provision_v2::{ensure_applied_from_packblob_bytes, DependencyProvider, LaunchPlan};
use runner_v2_rcon::{load_rcon_settings, RconClient};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::config;
use super::logs::LogStore;
use super::monitor::ensure_monitor;
use super::state::{ServerState, SharedState};
use super::updates::{ensure_watchers, sync_whitelist_to_root};
use super::util::{default_server_root, now_millis};

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
    let artifact = match hub.get_launcher_artifact(&deploy.pack_id, &deploy.channel, current_build_id.as_deref()).await {
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
    let env = BTreeMap::new();

    if let Err(err) = start_server(profile, &build, server_root.clone(), env, state).await {
        warn!("failed to auto-start server: {}", err.message);
        return;
    }

    // Save the new build_id and pack_blob
    if let Some(build_id) = artifact.build_id {
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
    env: BTreeMap<String, String>,
    state: SharedState,
) -> Result<Response, RpcError> {
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

    let launch_plan = apply_pack_blob(&server_root, pack_blob_bytes).await?;
    if let Ok(Some(deploy)) = config::load_deploy_key() {
        if let Ok(mut hub) = HubClient::new(&deploy.hub_url) {
            hub.set_service_token(deploy.deploy_key.clone());
            let hub = Arc::new(hub);
            if let Err(err) = sync_whitelist_to_root(hub, &deploy.pack_id, &server_root).await {
                warn!("whitelist sync failed on start: {err}");
            }
        }
    }
    let logs = {
        let guard = state.lock().await;
        guard.logs.clone()
    };
    let spawn_logs = logs.clone();
    let child = spawn_server(&launch_plan, &server_root, &env, spawn_logs).await.map_err(|err| RpcError {
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
        guard.env = env.clone();
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

    ensure_watchers(state.clone()).await;
    ensure_monitor(state.clone()).await;

    Ok(Response::Started { profile, pid, started_at_ms })
}

pub async fn stop_server(force: bool, state: SharedState) -> Result<Response, RpcError> {
    let mut guard = state.lock().await;
    refresh_child_status(&mut guard).await;
    if guard.child.is_none() {
        return Err(RpcError {
            code: ErrorCode::ServerNotRunning,
            message: "server not running".into(),
            details: Default::default(),
        });
    }

    guard.restart_disabled = true;
    drop(guard);

    stop_server_internal(state.clone(), force).await?;

    let mut guard = state.lock().await;
    let profile = guard.profile.clone().unwrap_or_else(|| "default".into());
    let stopped_at_ms = now_millis();
    let exit_info = ExitInfo { code: None, signal: None };
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
    ensure_applied_from_packblob_bytes(server_root, pack_blob, &provider)
        .await
        .map_err(|err| RpcError {
            code: ErrorCode::InvalidConfig,
            message: format!("provision failed: {err}"),
            details: Default::default(),
        })
}

pub(crate) async fn stop_server_internal(state: SharedState, force: bool) -> Result<(), RpcError> {
    let server_root = super::util::current_server_root(&state).await.ok_or_else(|| RpcError {
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
            for _ in 0..20 {
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
    let program = argv.next().ok_or_else(|| std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "empty launch command",
    ))?;

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
        let response = self.client.get(&dep.url)
            .send()
            .await
            .map_err(|err| {
                runner_provision_v2::errors::ProvisionError::Invalid(
                    format!("dependency download failed: {err}"),
                )
            })?
            .error_for_status()
            .map_err(|err| {
                runner_provision_v2::errors::ProvisionError::Invalid(
                    format!("dependency download failed: {err}"),
                )
            })?;
        let bytes = response
            .bytes()
            .await
            .map_err(|err| {
                runner_provision_v2::errors::ProvisionError::Invalid(
                    format!("dependency download failed: {err}"),
                )
            })?;
        Ok(bytes.to_vec())
    }
}

async fn load_current_build_id(server_root: &PathBuf) -> Option<String> {
    let path = server_root.join("current").join(".runner").join("build_id.txt");
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
    let path = server_root.join("current").join(".runner").join("pack_blob.bin");
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
