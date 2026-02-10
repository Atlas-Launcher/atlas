use futures_util::StreamExt;
use runner_core_v2::proto::{Envelope, Request, Response};
use runner_core_v2::PROTOCOL_VERSION;
use crate::client::connect_or_start;

pub async fn ping() -> anyhow::Result<String> {
    let mut framed = connect_or_start().await?;

    let req = Envelope {
        id: 1,
        payload: Request::Ping {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
        },
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = runner_ipc_v2::framing::read_response(&mut framed).await?;

    match resp.payload {
        Response::Pong { daemon_version, protocol_version } => {
            Ok(format!("pong: daemon={daemon_version} protocol={protocol_version}"))
        }
        other => Ok(format!("unexpected: {other:?}")),
    }
}

pub async fn shutdown() -> anyhow::Result<String> {
    let mut framed = connect_or_start().await?;

    let req = Envelope {
        id: 1,
        payload: Request::Shutdown {},
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = runner_ipc_v2::framing::read_response(&mut framed).await?;

    match resp.payload {
        Response::ShutdownAck {} => {
            Ok(format!("Daemon acknowledged shutdown request."))
        }
        other => Ok(format!("unexpected: {other:?}")),
    }
}