use tokio::net::UnixListener;
use runner_core_v2::proto::*;
use runner_ipc_v2::framing;
use std::process;

pub async fn serve(listener: UnixListener) -> std::io::Result<()> {
    loop {
        let (stream, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = handle_conn(stream).await;
        });
    }
}

async fn handle_conn(stream: tokio::net::UnixStream) -> std::io::Result<()> {
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
