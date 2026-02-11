use crate::client::connect_or_start;
use atlas_client::hub::HubClient;
use runner_core_v2::proto::{Envelope, Outbound, Request, Response};
use runner_core_v2::PROTOCOL_VERSION;
use runner_v2_utils::{ensure_dir, runtime_paths_v2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use sysinfo::System;

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
        Response::Pong {
            daemon_version,
            protocol_version,
        } => Ok(format!(
            "pong: daemon={daemon_version} protocol={protocol_version}"
        )),
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
        Response::ShutdownAck {} => Ok(format!("Daemon acknowledged shutdown request.")),
        other => Ok(format!("unexpected: {other:?}")),
    }
}

pub async fn up(
    profile: String,
    pack_blob: Option<PathBuf>,
    server_root: Option<PathBuf>,
    max_ram: Option<u32>,
    accept_eula: bool,
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
        let blob_path = paths
            .runtime_dir
            .join(format!("pack-{}-{}.bin", config.pack_id, config.channel));
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

    if let Ok(mut config) = load_deploy_key() {
        if config.eula_accepted != Some(true) {
            if accept_eula {
                config.eula_accepted = Some(true);
            } else {
                println!("Minecraft EULA: https://aka.ms/MinecraftEULA");
                print!("Do you accept the Minecraft EULA? (y/N): ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let accepted = tokio::task::spawn_blocking(|| {
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).unwrap();
                    let answer = input.trim().to_ascii_lowercase();
                    answer == "y" || answer == "yes"
                })
                .await
                .unwrap();
                if !accepted {
                    anyhow::bail!("EULA not accepted. Re-run with --accept-eula to proceed.");
                }
                config.eula_accepted = Some(true);
            }
        }
        if config.max_ram.is_none() {
            // first run, prompt for channel
            println!("Select channel to follow:");
            println!("1. production (stable releases)");
            println!("2. beta (pre-release testing)");
            println!("3. dev (latest development builds)");
            print!("Enter choice (1-3, default 1): ");
            std::io::Write::flush(&mut std::io::stdout())?;
            let channel = tokio::task::spawn_blocking(|| {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let choice = input.trim().parse::<u32>().unwrap_or(1);
                match choice {
                    1 => "production".to_string(),
                    2 => "beta".to_string(),
                    3 => "dev".to_string(),
                    _ => "production".to_string(),
                }
            })
            .await
            .unwrap();
            config.channel = channel;
        }
        let max_ram_val_mb = if let Some(arg_val_mb) = max_ram {
            arg_val_mb.max(512)
        } else if let Some(existing) = config.max_ram {
            normalize_max_ram_mb(existing)
        } else {
            // prompt
            let default_mb = get_default_max_ram_mb();
            let default_gb = default_mb.div_ceil(1024);
            println!(
                "Default RAM limit is {} GB ({} MB). Override? (y/n)",
                default_gb, default_mb
            );
            let do_override = tokio::task::spawn_blocking(|| {
                use std::io::{self};
                let stdin = io::stdin();
                let mut input = String::new();
                stdin.read_line(&mut input).unwrap();
                input.trim().to_lowercase() == "y"
            })
            .await
            .unwrap();
            if do_override {
                println!("Enter RAM limit in GB:");
                let ram_gb = tokio::task::spawn_blocking(move || {
                    use std::io::{self};
                    let stdin = io::stdin();
                    let mut input = String::new();
                    stdin.read_line(&mut input).unwrap();
                    input.trim().parse::<u32>().unwrap_or(default_gb)
                })
                .await
                .unwrap();
                (ram_gb.max(1)) * 1024
            } else {
                default_mb
            }
        };
        config.max_ram = Some(max_ram_val_mb);
        config.should_autostart = Some(true);
        let _ = save_deploy_key(&config);
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
    #[serde(default)]
    first_run: Option<bool>,
}

fn save_deploy_key(config: &DeployKeyConfig) -> anyhow::Result<()> {
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
    Err(anyhow::anyhow!(
        "Unable to resolve a writable data directory"
    ))
}

fn get_default_max_ram_mb() -> u32 {
    let mut system = System::new();
    system.refresh_memory();
    let total_gb = (system.total_memory() / 1024 / 1024 / 1024) as u32;
    let target_gb = if total_gb <= 8 {
        (total_gb.saturating_sub(2)).max(1)
    } else {
        #[cfg(target_os = "macos")]
        {
            8
        }
        #[cfg(target_os = "linux")]
        {
            if total_gb == 16 {
                14
            } else if total_gb >= 24 {
                16
            } else {
                8
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            8
        }
    };
    target_gb * 1024
}

fn normalize_max_ram_mb(value: u32) -> u32 {
    if value == 0 {
        return get_default_max_ram_mb();
    }
    // Backward compatibility: previous clients stored GB-like values.
    if value <= 64 {
        return value * 1024;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::normalize_max_ram_mb;

    #[test]
    fn normalizes_legacy_gb_values_to_mb() {
        assert_eq!(normalize_max_ram_mb(8), 8192);
        assert_eq!(normalize_max_ram_mb(16), 16384);
    }

    #[test]
    fn preserves_mb_values() {
        assert_eq!(normalize_max_ram_mb(4096), 4096);
    }
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
