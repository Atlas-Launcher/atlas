use std::process;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::warn;

use runner_core_v2::proto::*;
use runner_ipc_v2::framing;

use crate::config::{save_deploy_key, DeployKeyConfig};
use crate::supervisor::{
    build_status,
    ensure_rcon_available,
    ensure_watchers,
    execute_rcon_command,
    start_server,
    start_server_from_deploy,
    stop_server,
    LogStore,
    ServerState,
    SharedState,
};

pub async fn serve(listener: UnixListener, logs: LogStore) -> std::io::Result<()> {
    let state: SharedState = Arc::new(Mutex::new(ServerState::new(logs)));
    let start_ms = crate::supervisor::now_millis();
    let auto_state = state.clone();
    tokio::spawn(async move {
        start_server_from_deploy(auto_state).await;
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
                    let payload = match start_server(profile, env, state).await {
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
                let payload = Response::LogsTail {
                    lines: logs.tail_server(lines),
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
                };

                match save_deploy_key(&config) {
                    Ok(()) => {
                        ensure_watchers(state.clone()).await;
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
