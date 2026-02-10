use std::process;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{warn, info};
use tokio::signal;

use runner_core_v2::proto::*;
use runner_ipc_v2::framing;

use crate::config::{save_deploy_key, DeployKeyConfig};
use crate::supervisor::{
    build_status,
    ensure_rcon_available,
    ensure_watchers,
    execute_rcon_command,
    start_server_from_deploy,
    stop_server,
    LogStore,
    ServerState,
    SharedState,
    default_server_root,
};

pub async fn serve(listener: UnixListener, logs: LogStore) -> std::io::Result<()> {
    let state: SharedState = Arc::new(Mutex::new(ServerState::new(logs)));
    let start_ms = crate::supervisor::now_millis();
    let auto_state = state.clone();
    // Run auto-start synchronously in this task to avoid requiring `start_server_from_deploy` to be Send.
    start_server_from_deploy(auto_state).await;

    // Signal handler for SIGTERM (graceful shutdown)
    let state_for_signal = state.clone();
    tokio::spawn(async move {
        // Wait for SIGTERM
        signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap().recv().await;
        info!("Received SIGTERM, stopping Minecraft server gracefully...");
        if let Err(err) = crate::supervisor::stop_server(false, state_for_signal.clone()).await {
            warn!("SIGTERM graceful shutdown failed: {}", err.message);
            let _ = crate::supervisor::stop_server(true, state_for_signal).await;
        }
        info!("Graceful shutdown complete. Exiting daemon.");
        std::process::exit(0);
    });

    // Signal handler for SIGINT: attempt graceful shutdown on first Ctrl-C, escalate on subsequent presses
    let state_for_sigint = state.clone();
    tokio::spawn(async move {
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt()).unwrap();
        use std::time::{Instant, Duration as StdDuration};
        let mut last = None::<Instant>;
        let mut count = 0usize;
        // define a window in which successive Ctrl-C presses count towards escalation
        let window = StdDuration::from_secs(10);

        loop {
            // wait for a SIGINT
            sigint.recv().await;
            let now = Instant::now();
            if let Some(prev) = last {
                if now.duration_since(prev) > window {
                    count = 0; // reset if outside the window
                }
            }
            last = Some(now);
            count = count.saturating_add(1);

            if count == 1 {
                info!("Received SIGINT (Ctrl-C): attempting graceful shutdown. Press Ctrl-C two more times within 10s to force kill.");
                let state_clone = state_for_sigint.clone();
                tokio::spawn(async move {
                    if let Err(err) = crate::supervisor::stop_server(false, state_clone.clone()).await {
                        warn!("SIGINT graceful shutdown failed: {}", err.message);
                        let _ = crate::supervisor::stop_server(true, state_clone).await;
                    }
                    info!("Graceful shutdown complete. Exiting daemon.");
                    std::process::exit(0);
                });
            } else if count == 2 {
                warn!("Received second Ctrl-C: press once more to force immediate kill.");
            } else {
                // third (or more) Ctrl-C within window -> force kill immediately
                warn!("Received Ctrl-C {} times: force killing Minecraft server...", count);
                let mut guard = state_for_sigint.lock().await;
                if let Some(child) = guard.child.as_mut() {
                    let _ = child.kill().await;
                    info!("Minecraft server process killed.");
                }
                process::exit(1);
            }
        }
    });

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
    state: SharedState,
    daemon_start_ms: u64,
) -> std::io::Result<()> {
    let mut framed = framing::framed(stream);
    let (resp_tx, mut resp_rx) = tokio::sync::mpsc::channel::<PendingOutbound>(32);

    // Per-connection session state. Simplest possible.
    let mut next_session_id: SessionId = 1;
    let mut active_session: Option<SessionId> = None;

    loop {
        tokio::select! {
            outbound = resp_rx.recv() => {
                let Some(outbound) = outbound else {
                    break;
                };
                match outbound {
                    PendingOutbound::Send(out) => {
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                    PendingOutbound::SendAndExit(out) => {
                        framing::send_outbound(&mut framed, &out).await?;
                        process::exit(0);
                    }
                }
            }
            req_env = framing::read_request(&mut framed) => {
                let Some(req_env) = req_env? else {
                    break;
                };
        let req_id = req_env.id;

        match req_env.payload {
            Request::Shutdown {} => {
                let tx = resp_tx.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(err) = stop_server(false, state.clone()).await {
                        warn!("shutdown stop failed: {}", err.message);
                        let _ = stop_server(true, state).await;
                    }
                    let resp = Response::ShutdownAck {};
                    let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                    let _ = tx.send(PendingOutbound::SendAndExit(out)).await;
                });
            }

            Request::Ping { protocol_version, .. } => {
                let resp = Response::Pong {
                    daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                    protocol_version,
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
                let tx = resp_tx.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    let pack_blob_path = match env.get("ATLAS_PACK_BLOB") {
                        Some(path) if !path.trim().is_empty() => path.clone(),
                        _ => {
                            let err = Response::Error(RpcError {
                                code: ErrorCode::BadRequest,
                                message: "ATLAS_PACK_BLOB is required".into(),
                                details: Default::default(),
                            });
                            let out = Outbound::Response(Envelope { id: req_id, payload: err });
                            let _ = tx.send(PendingOutbound::Send(out)).await;
                            return;
                        }
                    };
                    let pack_blob_bytes = match tokio::fs::read(&pack_blob_path).await {
                        Ok(bytes) => bytes,
                        Err(err) => {
                            let err = Response::Error(RpcError {
                                code: ErrorCode::IoError,
                                message: format!("failed to read pack blob: {err}"),
                                details: Default::default(),
                            });
                            let out = Outbound::Response(Envelope { id: req_id, payload: err });
                            let _ = tx.send(PendingOutbound::Send(out)).await;
                            return;
                        }
                    };
                    let server_root = env
                        .get("ATLAS_SERVER_ROOT")
                        .map(|value| std::path::PathBuf::from(value))
                        .unwrap_or_else(|| default_server_root(&profile));
                    let payload = match crate::supervisor::start_server(profile, &pack_blob_bytes, server_root, state).await {
                        Ok(resp) => resp,
                        Err(err) => Response::Error(err),
                    };
                    let out = Outbound::Response(Envelope { id: req_id, payload });
                    let _ = tx.send(PendingOutbound::Send(out)).await;
                });
            }

            Request::Stop { force, .. } => {
                let tx = resp_tx.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    let payload = match stop_server(force, state).await {
                        Ok(resp) => resp,
                        Err(err) => Response::Error(err),
                    };
                    let out = Outbound::Response(Envelope { id: req_id, payload });
                    let _ = tx.send(PendingOutbound::Send(out)).await;
                });
            }

            Request::LogsTail { lines } => {
                let logs = {
                    let guard = state.lock().await;
                    guard.logs.clone()
                };
                let mut log_lines = logs.tail_server(lines);
                if log_lines.is_empty() {
                    log_lines.push(runner_core_v2::proto::LogLine {
                        at_ms: crate::supervisor::now_millis(),
                        stream: runner_core_v2::proto::LogStream::Stdout,
                        line: "No log content available yet. Server may not have started.".to_string(),
                    });
                }
                let payload = Response::LogsTail {
                    lines: log_lines,
                    truncated: false,
                };
                let out = Outbound::Response(Envelope { id: req_id, payload });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::DaemonLogsTail { lines } => {
                let logs = {
                    let guard = state.lock().await;
                    guard.logs.clone()
                };
                let payload = Response::LogsTail {
                    lines: logs.tail_daemon(lines),
                    truncated: false,
                };
                let out = Outbound::Response(Envelope { id: req_id, payload });
                framing::send_outbound(&mut framed, &out).await?;
            }

            Request::RconExec { command } => {
                match execute_rcon_command(&state, &command).await {
                    Ok(text) => {
                        let resp = Response::RconResult { text };
                        let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                    Err(err) => {
                        let out = Outbound::Response(Envelope {
                            id: req_id,
                            payload: Response::Error(RpcError {
                                code: ErrorCode::InvalidConfig,
                                message: err,
                                details: Default::default(),
                            }),
                        });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                }
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

                if let Err(err) = ensure_rcon_available(&state).await {
                    let out = Outbound::Response(Envelope {
                        id: req_id,
                        payload: Response::Error(RpcError {
                            code: ErrorCode::InvalidConfig,
                            message: err,
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

                match execute_rcon_command(&state, &command).await {
                    Ok(text) => {
                        let evt = Event::RconOut { session, text };
                        framing::send_outbound(&mut framed, &Outbound::Event(evt)).await?;
                    }
                    Err(err) => {
                        let evt = Event::RconErr { session, text: err };
                        framing::send_outbound(&mut framed, &Outbound::Event(evt)).await?;
                    }
                }

                let _ = req_id;
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

            Request::SaveDeployKey {
                hub_url,
                pack_id,
                channel,
                deploy_key,
                prefix,
            } => {
                let config = DeployKeyConfig {
                    hub_url,
                    pack_id,
                    channel,
                    deploy_key,
                    prefix,
                    max_ram: None,
                    should_autostart: None,
                    eula_accepted: None,
                };

                match save_deploy_key(&config) {
                    Ok(()) => {
                        // ensure_watchers may be non-Send; run it on a current-thread runtime inside spawn_blocking
                        let state_for_watchers = state.clone();
                        tokio::task::spawn_blocking(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .expect("failed to create local runtime for ensure_watchers");
                            rt.block_on(async move {
                                let _ = ensure_watchers(state_for_watchers).await;
                            });
                        });
                        let resp = Response::DeployKeySaved {};
                        let out = Outbound::Response(Envelope { id: req_id, payload: resp });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                    Err(err) => {
                        let out = Outbound::Response(Envelope {
                            id: req_id,
                            payload: Response::Error(RpcError {
                                code: ErrorCode::InvalidConfig,
                                message: err,
                                details: Default::default(),
                            }),
                        });
                        framing::send_outbound(&mut framed, &out).await?;
                    }
                }
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
        }
    }

    Ok(())
}

enum PendingOutbound {
    Send(Outbound),
    SendAndExit(Outbound),
}
