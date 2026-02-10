use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::process::Command;
use tokio::sync::Mutex;

use runner_core_v2::proto::*;
use runner_ipc_v2::framing;
use runner_v2_utils::runtime_paths_v2;
use runner_provision_v2::{ensure_applied_from_packblob_bytes, DependencyProvider, LaunchPlan};

pub async fn serve(listener: UnixListener) -> std::io::Result<()> {
    let state = Arc::new(Mutex::new(ServerState::new()));
    let start_ms = now_millis();
    loop {
        let (stream, _addr) = listener.accept().await?;
        let state = Arc::clone(&state);
        let start_ms = start_ms;
        tokio::spawn(async move {
            let _ = handle_conn(stream, state, start_ms).await;
        });
    }
}

async fn handle_conn(
    stream: tokio::net::UnixStream,
    state: Arc<Mutex<ServerState>>,
    daemon_start_ms: u64,
) -> std::io::Result<()> {
    let mut framed = framing::framed(stream);

    // Per-connection session state. Simplest possible.
    let mut next_session_id: SessionId = 1;
    let mut active_session: Option<SessionId> = None;

    while let Some(req_env) = framing::read_request(&mut framed).await? {
        let req_id = req_env.id;

        match req_env.payload {
            Request::Shutdown {} => {
                let resp = Response::ShutdownAck {};
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
                process::exit(0);
            }

            Request::Ping { protocol_version, .. } => {
                // respond
                let resp = Response::Pong {
                    daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                    protocol_version, // or your constant
                };
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::Status {} => {
                let (daemon, server) = build_status(daemon_start_ms, &state).await;
                let resp = Response::Status { daemon, server };
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::Start { profile, env } => {
                let mut state_guard = state.lock().await;
                if state_guard.is_running() {
                    let out = Outbound::Response(Envelope {
                        id: req_id,
                        payload: Response::Error(RpcError {
                            code: ErrorCode::ServerAlreadyRunning,
                            message: "server already running".into(),
                            details: Default::default(),
                        }),
                    });
                    framing::send_outbound(&mut framed, &out).await?;
                    continue;
                }

                match start_server(profile, env, &mut state_guard).await {
                    Ok(resp) => {
                        let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                    Err(err) => {
                        let out = Outbound::Response(Envelope {
                            id: req_id,
                            payload: Response::Error(err),
                        });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                }
            }

            Request::Stop { force, .. } => {
                let mut state_guard = state.lock().await;
                match stop_server(force, &mut state_guard).await {
                    Ok(resp) => {
                        let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                    Err(err) => {
                        let out = Outbound::Response(Envelope {
                            id: req_id,
                            payload: Response::Error(err),
                        });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                }
            }

            Request::RconExec { command } => {
                // TODO: replace with real rcon call
                // let text = rcon.exec(&command).await?;
                let text = format!("(stub) executed: {command}");

                let resp = Response::RconResult { text };
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::RconOpen {} => {
                if active_session.is_some() {
                    let out = Outbound::Response(Envelope {
                        id: req_id,
                        payload: Response::Error(RpcError {
                            code: ErrorCode::BadRequest,
                            message: "RCON session already open on this connection".into(),
                            details: Default::default(),
                        }),
                    });
                    framing::send_outbound(&mut framed, &out).await?;
                    continue;
                }

                let sid = next_session_id;
                next_session_id += 1;
                active_session = Some(sid);

                let resp = Response::RconOpened { session: sid, prompt: "rcon> ".into() };
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::RconSend { session, command } => {
                if active_session != Some(session) {
                    let out = Outbound::Response(Envelope {
                        id: req_id,
                        payload: Response::Error(RpcError {
                            code: ErrorCode::BadRequest,
                            message: "invalid or inactive session".into(),
                            details: Default::default(),
                        }),
                    });
                    framing::send_outbound(&mut framed, &out).await?;
                    continue;
                }

                // Execute and stream output as an Event (so CLI can continuously print)
                // let text = rcon.exec(&command).await?;
                let text = format!("(stub) {command} -> ok");

                let evt = Event::RconOut { session, text };
                framing::send_outbound(&mut framed, &Outbound::Event(evt)).await?;

                // Optionally also send an ack as a Response (not strictly needed)
                // framing::send_outbound(&mut framed, &Outbound::Response(Envelope { id: req_id, payload: Response::RconAck { session } })).await?;
                let _ = req_id; // if you skip ack, keep id unused or remove it
            }

            Request::RconClose { session } => {
                if active_session != Some(session) {
                    let out = Outbound::Response(Envelope {
                        id: req_id,
                        payload: Response::Error(RpcError {
                            code: ErrorCode::BadRequest,
                            message: "invalid or inactive session".into(),
                            details: Default::default(),
                        }),
                    });
                    framing::send_outbound(&mut framed, &out).await?;
                    continue;
                }

                active_session = None;
                let resp = Response::RconClosed { session };
                let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                framing::send_outbound(&mut framed, &out).await?;
            }

            _ => {
                let out = Outbound::Response(Envelope {
                    id: req_id,
                    payload: Response::Error(RpcError {
                        code: ErrorCode::UnsupportedProtocol,
                        message: "unsupported request type".into(),
                        details: Default::default(),
                    }),
                });
                framing::send_outbound(&mut framed, &out).await?;
            }
        }
    }

    Ok(())
}

struct ServerState {
    status: ServerStatus,
    child: Option<tokio::process::Child>,
    profile: Option<ProfileId>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            status: ServerStatus::Idle {},
            child: None,
            profile: None,
        }
    }

    fn is_running(&self) -> bool {
        matches!(self.status, ServerStatus::Running { .. } | ServerStatus::Starting { .. })
    }
}

async fn build_status(daemon_start_ms: u64, state: &Arc<Mutex<ServerState>>) -> (DaemonStatus, ServerStatus) {
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

async fn start_server(
    profile: ProfileId,
    env: BTreeMap<String, String>,
    state: &mut ServerState,
) -> Result<Response, RpcError> {
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

    let provider = HttpDependencyProvider::default();
    let launch_plan = ensure_applied_from_packblob_bytes(&server_root, &pack_blob, &provider)
        .await
        .map_err(|err| RpcError {
            code: ErrorCode::InvalidConfig,
            message: format!("provision failed: {err}"),
            details: Default::default(),
        })?;

    let child = spawn_server(&launch_plan, &server_root, &env).await.map_err(|err| RpcError {
        code: ErrorCode::Internal,
        message: format!("failed to start server: {err}"),
        details: Default::default(),
    })?;

    let pid = child.id().unwrap_or_default() as i32;
    let started_at_ms = now_millis();

    state.child = Some(child);
    state.profile = Some(profile.clone());
    state.status = ServerStatus::Running {
        profile: profile.clone(),
        pid,
        started_at_ms,
        meta: Default::default(),
    };

    Ok(Response::Started {
        profile,
        pid,
        started_at_ms,
    })
}

async fn stop_server(force: bool, state: &mut ServerState) -> Result<Response, RpcError> {
    refresh_child_status(state).await;
    let Some(child) = state.child.as_mut() else {
        return Err(RpcError {
            code: ErrorCode::ServerNotRunning,
            message: "server not running".into(),
            details: Default::default(),
        });
    };

    if force {
        child.kill().await.map_err(|err| RpcError {
            code: ErrorCode::IoError,
            message: format!("failed to kill server: {err}"),
            details: Default::default(),
        })?;
    }

    let exit = child.wait().await.map_err(|err| RpcError {
        code: ErrorCode::IoError,
        message: format!("failed to wait for server: {err}"),
        details: Default::default(),
    })?;

    let exit_info = ExitInfo {
        code: exit.code(),
        signal: None,
    };

    let profile = state.profile.clone().unwrap_or_else(|| "default".into());
    let stopped_at_ms = now_millis();

    state.child = None;
    state.status = ServerStatus::Exited {
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

async fn spawn_server(
    plan: &LaunchPlan,
    server_root: &PathBuf,
    env: &BTreeMap<String, String>,
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
    for (key, value) in env {
        cmd.env(key, value);
    }

    cmd.spawn()
}

fn default_server_root(profile: &str) -> PathBuf {
    let paths = runtime_paths_v2();
    paths.runtime_dir.join("servers").join(profile)
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Default)]
struct HttpDependencyProvider;

#[async_trait::async_trait]
impl DependencyProvider for HttpDependencyProvider {
    async fn fetch(&self, dep: &protocol::Dependency) -> Result<Vec<u8>, runner_provision_v2::errors::ProvisionError> {
        let response = reqwest::get(&dep.url)
            .await
            .map_err(|err| runner_provision_v2::errors::ProvisionError::Invalid(format!("dependency download failed: {err}")))?
            .error_for_status()
            .map_err(|err| runner_provision_v2::errors::ProvisionError::Invalid(format!("dependency download failed: {err}")))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|err| runner_provision_v2::errors::ProvisionError::Invalid(format!("dependency download failed: {err}")))?;
        Ok(bytes.to_vec())
    }
}
