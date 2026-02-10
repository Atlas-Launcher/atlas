use futures_util::StreamExt;
use runner_core_v2::proto::{Envelope, Outbound, Request, Response};
use runner_core_v2::PROTOCOL_VERSION;
use std::collections::BTreeMap;
use std::path::PathBuf;
use crate::client::connect_or_start;
use atlas_client::hub::HubClient;
use runner_v2_utils::{ensure_dir, runtime_paths_v2};
use serde::Deserialize;

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
    let resp = read_response_payload(&mut framed).await?;

    match resp {
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
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::ShutdownAck {} => {
            Ok(format!("Daemon acknowledged shutdown request."))
        }
        other => Ok(format!("unexpected: {other:?}")),
    }
}

pub async fn up(
    profile: String,
    pack_blob: Option<PathBuf>,
    server_root: Option<PathBuf>,
) -> anyhow::Result<String> {
    let mut framed = connect_or_start().await?;

    let pack_blob_path = if let Some(path) = pack_blob {
        if !path.exists() {
            anyhow::bail!("pack blob not found: {}", path.display());
        }
        path
    } else {
        let config = load_deploy_key()?;
        let mut hub = HubClient::new(&config.hub_url)?;
        hub.set_service_token(config.deploy_key.clone());
        let build = hub.get_build_blob(&config.pack_id, &config.channel).await?;
        let paths = runtime_paths_v2();
        ensure_dir(&paths.runtime_dir)?;
        let blob_path = paths.runtime_dir.join(format!("pack-{}-{}.bin", config.pack_id, config.channel));
        tokio::fs::write(&blob_path, build.bytes).await?;
        blob_path
    };

    let mut env = BTreeMap::new();
    env.insert(
        "ATLAS_PACK_BLOB".to_string(),
        pack_blob_path.to_string_lossy().to_string(),
    );
    if let Some(root) = server_root {
        env.insert(
            "ATLAS_SERVER_ROOT".to_string(),
            root.to_string_lossy().to_string(),
        );
    }

    let req = Envelope {
        id: 1,
        payload: Request::Start { profile, env },
    };

    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;
    let resp = read_response_payload(&mut framed).await?;

    match resp {
        Response::Started { pid, .. } => Ok(format!("started server pid={pid}")),
        Response::Error(err) => Err(anyhow::anyhow!("start failed: {}", err.message)),
        other => Ok(format!("unexpected: {other:?}")),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct DeployKeyConfig {
    hub_url: String,
    pack_id: String,
    #[serde(default = "default_channel")]
    channel: String,
    deploy_key: String,
    #[serde(default)]
    prefix: Option<String>,
}

fn load_deploy_key() -> anyhow::Result<DeployKeyConfig> {
    let path = deploy_key_path()?;
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("Failed to read deploy key config: {err}"))?;
    let config = serde_json::from_str::<DeployKeyConfig>(&content)
        .map_err(|err| anyhow::anyhow!("Failed to parse deploy key config: {err}"))?;
    Ok(config)
}

fn deploy_key_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join("deploy.json"))
}

fn config_dir() -> anyhow::Result<PathBuf> {
    if let Some(base) = dirs::data_dir() {
        return Ok(base.join("atlas").join("runnerd"));
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".atlas").join("runnerd"));
    }
    Err(anyhow::anyhow!("Unable to resolve a writable data directory"))
}

fn default_channel() -> String {
    "production".to_string()
}

async fn read_response_payload(
    framed: &mut runner_ipc_v2::framing::FramedStream,
) -> anyhow::Result<Response> {
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