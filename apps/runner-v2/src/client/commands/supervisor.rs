use anyhow::Result;
use runner_core_v2::proto::{Envelope, ExitInfo, LogLine, Outbound, Request, Response};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::client::connect_or_start;

pub struct StopInfo {
    pub exit: Option<ExitInfo>,
    pub stopped_at_ms: u64,
}

pub struct LogsTailInfo {
    pub lines: Vec<LogLine>,
    pub truncated: bool,
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
        Response::Stopped {
            exit,
            stopped_at_ms,
        } => {
            if let Ok(mut config) = load_deploy_key() {
                config.should_autostart = Some(false);
                let _ = save_deploy_key(&config);
            }
            Ok(StopInfo {
                exit,
                stopped_at_ms,
            })
        }
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

pub async fn logs_tail_follow(lines: usize) -> Result<LogsTailInfo> {
    let mut framed = crate::client::connect_only().await?;
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

pub async fn daemon_logs_tail_follow(lines: usize) -> Result<LogsTailInfo> {
    let mut framed = crate::client::connect_only().await?;
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DeployKeyConfig {
    hub_url: String,
    pack_id: String,
    #[serde(default = "default_channel")]
    channel: String,
    deploy_key: String,
    #[serde(default)]
    prefix: Option<String>,
    #[serde(default)]
    max_ram: Option<u32>,
    #[serde(default)]
    should_autostart: Option<bool>,
    #[serde(default)]
    eula_accepted: Option<bool>,
}

fn save_deploy_key(config: &DeployKeyConfig) -> Result<()> {
    let path = deploy_key_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| anyhow::anyhow!("Failed to create runnerd config dir: {err}"))?;
    }

    let payload = serde_json::to_string_pretty(config)
        .map_err(|err| anyhow::anyhow!("Failed to serialize deploy key config: {err}"))?;
    std::fs::write(&path, payload)
        .map_err(|err| anyhow::anyhow!("Failed to write deploy key config: {err}"))?;

    Ok(())
}

fn load_deploy_key() -> Result<DeployKeyConfig> {
    let path = deploy_key_path()?;
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("Failed to read deploy key config: {err}"))?;
    let config = serde_json::from_str::<DeployKeyConfig>(&content)
        .map_err(|err| anyhow::anyhow!("Failed to parse deploy key config: {err}"))?;
    Ok(config)
}

fn deploy_key_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("deploy.json"))
}

fn config_dir() -> Result<PathBuf> {
    if let Some(base) = dirs::data_dir() {
        return Ok(base.join("atlas").join("runnerd"));
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".atlas").join("runnerd"));
    }
    Err(anyhow::anyhow!(
        "Unable to resolve a writable data directory"
    ))
}

fn default_channel() -> String {
    "production".to_string()
}
