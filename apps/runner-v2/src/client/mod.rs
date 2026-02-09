use std::time::Duration;

use runner_core_v2::PROTOCOL_VERSION;
use runner_core_v2::proto::{Envelope, Request, Response};
use runner_v2_utils::{ensure_dir, runtime_paths_v2};

use tokio::process::Command;

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

async fn connect_or_start() -> anyhow::Result<runner_ipc_v2::framing::FramedStream> {
    let paths = runtime_paths_v2();
    ensure_dir(&paths.runtime_dir)?;

    if let Ok(stream) = runner_ipc_v2::socket::connect(&paths.socket_path).await {
        return Ok(runner_ipc_v2::framing::framed(stream));
    }

    start_daemon_detached().await?;

    for _ in 0..30 {
        if let Ok(stream) = runner_ipc_v2::socket::connect(&paths.socket_path).await {
            return Ok(runner_ipc_v2::framing::framed(stream));
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    anyhow::bail!("failed to connect to runnerd2 after starting it");
}

async fn start_daemon_detached() -> anyhow::Result<()> {
    // In dev, run runnerd2 in another terminal or install it so it’s on PATH.
    Command::new("runnerd2")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}
