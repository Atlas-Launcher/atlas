use anyhow::Result;
use runner_core_v2::proto::{DaemonStatus, Envelope, ExitInfo, LogLine, Outbound, Request, Response, ServerStatus};

use crate::client::connect_or_start;

pub struct StatusInfo {
    pub daemon: DaemonStatus,
    pub server: ServerStatus,
}

pub struct StopInfo {
    pub exit: Option<ExitInfo>,
    pub stopped_at_ms: u64,
}

pub struct LogsTailInfo {
    pub lines: Vec<LogLine>,
    pub truncated: bool,
}

pub async fn status() -> Result<StatusInfo> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::Status {},
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::Status { daemon, server } => Ok(StatusInfo { daemon, server }),
        Response::Error(err) => Err(anyhow::anyhow!("status failed: {}", err.message)),
        other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
    }
}

pub async fn stop(force: bool) -> Result<StopInfo> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::Stop {
            force,
            grace_ms: None,
        },
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::Stopped { exit, stopped_at_ms } => Ok(StopInfo { exit, stopped_at_ms }),
        Response::Error(err) => Err(anyhow::anyhow!("stop failed: {}", err.message)),
        other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
    }
}

pub async fn logs_tail(lines: usize) -> Result<LogsTailInfo> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::LogsTail { lines },
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::LogsTail { lines, truncated } => Ok(LogsTailInfo { lines, truncated }),
        Response::Error(err) => Err(anyhow::anyhow!("logs tail failed: {}", err.message)),
        other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
    }
}

pub async fn daemon_logs_tail(lines: usize) -> Result<LogsTailInfo> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::DaemonLogsTail { lines },
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::LogsTail { lines, truncated } => Ok(LogsTailInfo { lines, truncated }),
        Response::Error(err) => Err(anyhow::anyhow!("daemon logs tail failed: {}", err.message)),
        other => Err(anyhow::anyhow!("unexpected response: {other:?}")),
    }
}

async fn read_response_payload(
    framed: &mut runner_ipc_v2::framing::FramedStream,
) -> Result<Response> {
    loop {
        let outbound = runner_ipc_v2::framing::read_outbound(framed)
            .await?
            .ok_or_else(|| anyhow::anyhow!("runnerd closed the connection"))?;
        match outbound {
            Outbound::Response(env) => return Ok(env.payload),
            Outbound::Event(_) => continue,
        }
    }
}
