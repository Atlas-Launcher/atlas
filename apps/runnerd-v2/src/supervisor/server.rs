use std::collections::BTreeMap;
use std::path::PathBuf;

use runner_core_v2::proto::*;
use runner_provision_v2::{ensure_applied_from_packblob_bytes, DependencyProvider, LaunchPlan};
use runner_v2_rcon::{load_rcon_settings, RconClient};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::{sleep, Duration};

use super::logs::LogStore;
use super::monitor::ensure_monitor;
use super::state::{ServerState, SharedState};
use super::updates::ensure_watchers;
use super::util::{default_server_root, now_millis};

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

    let pack_blob_path = env
        .get("ATLAS_PACK_BLOB")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RpcError {
            code: ErrorCode::BadRequest,
            message: "ATLAS_PACK_BLOB is required".into(),
            details: Default::default(),
        })?;

    let server_root = env
        .get("ATLAS_SERVER_ROOT")
        .map(|value| PathBuf::from(value))
        .unwrap_or_else(|| default_server_root(&profile));

    let pack_blob = tokio::fs::read(&pack_blob_path)
        .await
        .map_err(|err| RpcError {
            code: ErrorCode::IoError,
            message: format!("failed to read pack blob: {err}"),
            details: Default::default(),
        })?;

    let launch_plan = apply_pack_blob(&server_root, &pack_blob).await?;
    let logs = {
        let guard = state.lock().await;
        guard.logs.clone()
    };
    let child = spawn_server(&launch_plan, &server_root, &env, logs).await.map_err(|err| RpcError {
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

#[derive(Default)]
struct HttpDependencyProvider;

#[async_trait::async_trait]
impl DependencyProvider for HttpDependencyProvider {
    async fn fetch(
        &self,
        dep: &protocol::Dependency,
    ) -> Result<Vec<u8>, runner_provision_v2::errors::ProvisionError> {
        let response = reqwest::get(&dep.url)
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
