use clap::{Parser, Subcommand};
use atlas_client::hub::{DistributionReleaseAsset, DistributionReleaseResponse, HubClient};
use runner_core_v2::proto::{LogLine, LogStream};
use runner_v2_utils::runtime_paths_v2;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::time::{sleep, Duration};

mod client;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Auth {
        #[arg(long, value_name = "HUB_URL")]
        hub_url: Option<String>,

        #[arg(long, value_name = "PACK_ID")]
        pack_id: Option<String>,

        #[arg(long, value_name = "TOKEN_NAME")]
        token_name: Option<String>,

        #[arg(long, value_name = "CHANNEL", default_value = "production")]
        channel: String,
    },
    Ping,
    Shutdown,
    Down {
        #[arg(long)]
        force: bool,
    },
    Up {
        #[arg(long, value_name = "PROFILE", default_value = "default")]
        profile: String,

        #[arg(long, value_name = "PACK_BLOB")]
        pack_blob: Option<PathBuf>,

        #[arg(long, value_name = "SERVER_ROOT")]
        server_root: Option<PathBuf>,

        #[arg(long, value_name = "MAX_RAM_MB")]
        max_ram: Option<u32>,

        #[arg(long, default_value_t = false)]
        accept_eula: bool,
    },
    Exec {
        #[arg(short = 'i', long = "interactive")]
        interactive: bool,

        command: Option<String>,
    },
    Logs {
        #[arg(short = 'n', long = "lines", default_value_t = 200)]
        lines: usize,

        #[arg(short = 'f', long = "follow")]
        follow: bool,

        #[arg(long = "daemon-logs")]
        daemon_logs: bool,
    },
    Cd {
        #[arg(long, value_name = "SERVER_ROOT")]
        server_root: Option<PathBuf>,
    },
    Install {
        #[arg(long, value_name = "USER")]
        user: Option<String>,

        #[arg(long, value_name = "RUNNERD_PATH")]
        runnerd_path: Option<PathBuf>,
    },
    Backup,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Auth {
            hub_url,
            pack_id,
            token_name,
            channel,
        } => {
            let resp = client::auth(hub_url, pack_id, token_name, Some(channel)).await?;
            println!("{resp}");
        }
        Cmd::Ping => {
            let resp = client::ping().await?;
            println!("{resp}");
        }
        Cmd::Shutdown => {
            let resp = client::shutdown().await?;
            println!("{resp}");
        }
        Cmd::Down { force } => {
            let resp = client::stop(force).await?;
            if let Some(exit) = resp.exit {
                println!(
                    "stopped at {} (exit code: {:?})",
                    resp.stopped_at_ms, exit.code
                );
            } else {
                println!("stopped at {}", resp.stopped_at_ms);
            }
        }
        Cmd::Up {
            profile,
            pack_blob,
            server_root,
            max_ram,
            accept_eula,
        } => {
            if profile != "default" {
                eprintln!("Ignoring --profile; runner uses a single profile: default");
            }
            let resp = client::up(
                "default".to_string(),
                pack_blob,
                server_root,
                max_ram,
                accept_eula,
            )
            .await?;
            println!("{resp}");
        }
        Cmd::Exec {
            interactive,
            command,
        } => {
            let framed = client::connect_or_start().await?;
            if interactive {
                client::rcon_interactive(framed)
                    .await
                    .map_err(|e| anyhow::anyhow!("interactive rcon failed: {e}"))?;
            } else {
                let cmd = command
                    .ok_or_else(|| anyhow::anyhow!("command required for non-interactive exec"))?;
                client::rcon_exec(framed, cmd).await?;
            }
        }
        Cmd::Logs {
            lines,
            follow,
            daemon_logs,
        } => {
            if follow {
                follow_logs(lines, daemon_logs).await?;
            } else {
                let resp = if daemon_logs {
                    client::daemon_logs_tail(lines).await?
                } else {
                    client::logs_tail(lines).await?
                };
                for line in resp.lines {
                    print_log_line(&line);
                }
                if resp.truncated {
                    eprintln!("log output was truncated; use --lines or --follow for more output");
                }
            }
        }
        Cmd::Cd { server_root } => {
            let root = resolve_server_root(server_root);
            println!("{}", root.display());
        }
        Cmd::Install { user, runnerd_path } => {
            install_systemd(user, runnerd_path).await?;
            println!("atlas-runnerd systemd service enabled and started.");
        }
        Cmd::Backup => {
            let path = client::backup::backup_now().await?;
            println!("backup created: {}", path);
        }
    }
    Ok(())
}

async fn follow_logs(lines: usize, daemon_logs: bool) -> anyhow::Result<()> {
    let mut last_at_ms = 0u64;
    let mut last_lines: Vec<String> = Vec::new();

    loop {
        let resp = if daemon_logs {
            match client::daemon_logs_tail_follow(lines).await {
                Ok(resp) => resp,
                Err(err) => {
                    eprintln!("Daemon connection lost: {}", err);
                    break;
                }
            }
        } else {
            match client::logs_tail_follow(lines).await {
                Ok(resp) => resp,
                Err(err) => {
                    eprintln!("Daemon connection lost: {}", err);
                    break;
                }
            }
        };
        for line in resp.lines {
            if line.at_ms > last_at_ms {
                last_at_ms = line.at_ms;
                last_lines.clear();
                last_lines.push(line.line.clone());
                print_log_line(&line);
                continue;
            }

            if line.at_ms == last_at_ms && !last_lines.contains(&line.line) {
                last_lines.push(line.line.clone());
                print_log_line(&line);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}

fn print_log_line(line: &LogLine) {
    let stream = match line.stream {
        LogStream::Stdout => "stdout",
        LogStream::Stderr => "stderr",
    };
    println!("[{}] {}", stream, line.line.trim_end());
}

fn resolve_server_root(server_root: Option<PathBuf>) -> PathBuf {
    if let Some(value) = server_root {
        return value;
    }
    let paths = runtime_paths_v2();
    paths.runtime_dir.join("servers").join("default")
}

async fn install_systemd(user: Option<String>, runnerd_path: Option<PathBuf>) -> anyhow::Result<()> {
    if std::env::consts::OS != "linux" {
        anyhow::bail!("systemd install is only supported on Linux");
    }

    let service_user = user.unwrap_or_else(current_user);
    let runnerd = if let Some(path) = runnerd_path.as_ref() {
        path.display().to_string()
    } else {
        let downloaded = download_runnerd_via_distribution_api().await?;
        downloaded.display().to_string()
    };

    let service_path = Path::new("/etc/systemd/system/atlas-runnerd.service");
    let contents = format!(
        "[Unit]\n\
Description=Atlas Runner Daemon\n\
After=network-online.target\n\
Wants=network-online.target\n\n\
[Service]\n\
Type=simple\n\
User={service_user}\n\
ExecStart={runnerd}\n\
Restart=always\n\
RestartSec=5\n\
Environment=RUST_LOG=info\n\n\
[Install]\n\
WantedBy=multi-user.target\n"
    );

    std::fs::write(service_path, contents)
        .map_err(|err| anyhow::anyhow!("Failed to write {}: {err}", service_path.display()))?;

    run_systemctl(["daemon-reload"])?;
    run_systemctl(["enable", "--now", "atlas-runnerd.service"])?;
    Ok(())
}

async fn download_runnerd_via_distribution_api() -> anyhow::Result<PathBuf> {
    let hub_url = resolve_install_hub_url()?;
    let mut hub = HubClient::new(&hub_url)?;
    if let Ok(token) = std::env::var("ATLAS_TOKEN") {
        if !token.trim().is_empty() {
            hub.set_token(token);
        }
    }

    let arch = normalize_distribution_arch(std::env::consts::ARCH)?;
    let release = hub
        .get_latest_distribution_release("runnerd", "linux", arch)
        .await?;
    let asset = select_runnerd_asset(&release)?;
    let bytes = hub.download_distribution_asset(&asset.download_id).await?;

    let install_path = PathBuf::from("/usr/local/bin/atlas-runnerd");
    std::fs::write(&install_path, &bytes).map_err(|err| {
        anyhow::anyhow!(
            "Failed to write runnerd binary to {}: {err}",
            install_path.display()
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&install_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|err| {
                anyhow::anyhow!(
                    "Failed to set executable permissions on {}: {err}",
                    install_path.display()
                )
            })?;
    }

    Ok(install_path)
}

fn select_runnerd_asset(release: &DistributionReleaseResponse) -> anyhow::Result<&DistributionReleaseAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.kind == "binary")
        .or_else(|| release.assets.iter().find(|asset| asset.kind == "installer"))
        .ok_or_else(|| anyhow::anyhow!("No runnerd binary/installer asset found in distribution release"))
}

fn resolve_install_hub_url() -> anyhow::Result<String> {
    if let Ok(url) = std::env::var("ATLAS_HUB_URL") {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let deploy_config_path = resolve_runnerd_deploy_config_path()?;
    let content = std::fs::read_to_string(&deploy_config_path).map_err(|err| {
        anyhow::anyhow!(
            "ATLAS_HUB_URL is not set and failed to read {}: {err}",
            deploy_config_path.display()
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|err| {
        anyhow::anyhow!(
            "ATLAS_HUB_URL is not set and failed to parse {}: {err}",
            deploy_config_path.display()
        )
    })?;

    let hub_url = value
        .get("hub_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "ATLAS_HUB_URL is not set and deploy config does not contain hub_url ({})",
                deploy_config_path.display()
            )
        })?;
    Ok(hub_url.to_string())
}

fn resolve_runnerd_deploy_config_path() -> anyhow::Result<PathBuf> {
    if let Some(base) = dirs::data_dir() {
        return Ok(base.join("atlas").join("runnerd").join("deploy.json"));
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".atlas").join("runnerd").join("deploy.json"));
    }
    Err(anyhow::anyhow!(
        "Unable to resolve deploy config path and ATLAS_HUB_URL is not set"
    ))
}

fn normalize_distribution_arch(arch: &str) -> anyhow::Result<&'static str> {
    match arch {
        "x86_64" | "amd64" => Ok("x64"),
        "aarch64" | "arm64" => Ok("arm64"),
        other => Err(anyhow::anyhow!(
            "Unsupported architecture '{other}' for runnerd distribution install"
        )),
    }
}

fn run_systemctl<const N: usize>(args: [&str; N]) -> anyhow::Result<()> {
    let status = Command::new("systemctl").args(args).status()?;
    if !status.success() {
        anyhow::bail!("systemctl failed with exit code: {status}");
    }
    Ok(())
}

fn current_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| "atlas".to_string())
}
