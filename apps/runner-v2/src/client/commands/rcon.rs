use anyhow::Context;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use runner_core_v2::proto::*;
use runner_ipc_v2::framing;

pub async fn rcon_exec(mut framed: framing::FramedStream, command: String) -> anyhow::Result<()> {
    let req = Envelope {
        id: 1,
        payload: Request::RconExec { command },
    };

    // send request as a Response outbound? no—client sends Request envelope directly
    // framing currently only has send_outbound; add a tiny helper or just serialize here:
    send_request(&mut framed, &req).await?;

    // read daemon outbound until we get the matching response
    while let Some(msg) = framing::read_outbound(&mut framed).await? {
        match msg {
            Outbound::Response(env) if env.id == 1 => {
                match env.payload {
                    Response::RconResult { text } => {
                        print!("{text}");
                        if !text.ends_with('\n') { println!(); }
                        return Ok(());
                    }
                    Response::Error(e) => anyhow::bail!("rcon error: {} ({:?})", e.message, e.code),
                    other => anyhow::bail!("unexpected response: {other:?}"),
                }
            }
            _ => {
                // ignore unrelated events/responses (shouldn't happen for simple exec)
            }
        }
    }

    anyhow::bail!("daemon closed connection");
}

pub async fn rcon_interactive(mut framed: framing::FramedStream) -> anyhow::Result<()> {
    // Open session
    let open_id = 1;
    let open_req = Envelope { id: open_id, payload: Request::RconOpen {} };
    send_request(&mut framed, &open_req).await?;

    let (session, prompt) = loop {
        let msg = framing::read_outbound(&mut framed).await?
            .context("daemon closed connection")?;
        match msg {
            Outbound::Response(env) if env.id == open_id => match env.payload {
                Response::RconOpened { session, prompt } => break (session, prompt),
                Response::Error(e) => anyhow::bail!("open failed: {} ({:?})", e.message, e.code),
                other => anyhow::bail!("unexpected: {other:?}"),
            },
            _ => {}
        }
    };

    // stdin reader
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    // For interactive mode, we can send RconSend without expecting a Response ack
    // (we’ll just print Events). If you later add acks, handle them similarly.
    loop {
        print!("{prompt}");
        // ensure prompt flush for interactive use
        use std::io::Write;
        std::io::stdout().flush().ok();

        tokio::select! {
            // user input
            line = lines.next_line() => {
                let line: Option<String> = line?;
                match line {
                    None => {
                        // EOF (Ctrl+D): close session
                        let close_req = Envelope { id: 9999, payload: Request::RconClose { session } };
                        let _ = send_request(&mut framed, &close_req).await;
                        return Ok(());
                    }
                    Some(mut s) => {
                        s = s.trim().to_string();
                        if s.is_empty() {
                            continue;
                        }
                        if s == "exit" || s == "quit" {
                            let close_req = Envelope { id: 9999, payload: Request::RconClose { session } };
                            send_request(&mut framed, &close_req).await?;
                            return Ok(());
                        }

                        let send_req = Envelope { id: 2, payload: Request::RconSend { session, command: s } };
                        send_request(&mut framed, &send_req).await?;
                    }
                }
            }

            // daemon output
            msg = framing::read_outbound(&mut framed) => {
                let msg: Outbound = msg?.context("daemon closed connection")?;
                match msg {
                    Outbound::Event(Event::RconOut { session: sid, text }) if sid == session => {
                        println!("{text}");
                    }
                    Outbound::Event(Event::RconErr { session: sid, text }) if sid == session => {
                        eprintln!("{text}");
                    }
                    // You may also see Response::RconClosed, Error, etc.
                    _ => {}
                }
            }
        }
    }
}

/// Client helper: send a Request envelope (client -> daemon).
async fn send_request(
    framed: &mut framing::FramedStream,
    req: &Envelope<Request>,
) -> std::io::Result<()> {
    use futures_util::SinkExt;
    let bytes = serde_json::to_vec(req)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    framed.send(bytes.into()).await
}
