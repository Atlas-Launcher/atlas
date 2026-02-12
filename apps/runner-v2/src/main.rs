use atlas_client::hub::{DistributionReleaseAsset, DistributionReleaseResponse, HubClient};
use clap::{Parser, Subcommand};
use runner_core_v2::proto::{LogLine, LogStream};
use runner_v2_utils::runtime_paths_v2;
use semver::Version;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::time::{sleep, Duration};

mod client;

#[derive(Parser)]
#[command(version = env!("ATLAS_BUILD_VERSION"))]
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

async fn install_systemd(
    user: Option<String>,
    runnerd_path: Option<PathBuf>,
) -> anyhow::Result<()> {
    if std::env::consts::OS != "linux" {
        anyhow::bail!("systemd install is only supported on Linux");
    }

    let service_user = user.unwrap_or_else(current_user);
    let runner_update = ensure_runner_binary_up_to_date().await?;
    if runner_update {
        println!("Updated /usr/local/bin/atlas-runner to the latest available version.");
    }

    let runnerd = if let Some(path) = runnerd_path.as_ref() {
        path.display().to_string()
    } else {
        let downloaded = download_runnerd_via_distribution_api().await?;
        downloaded.display().to_string()
    };

    let service_path = Path::new("/etc/systemd/system/atlas-runnerd.service");
    reconcile_runnerd_service_file(service_path, &runnerd, &service_user)?;

    run_systemctl(["daemon-reload"])?;
    run_systemctl(["enable", "--now", "atlas-runnerd.service"])?;
    Ok(())
}

async fn ensure_runner_binary_up_to_date() -> anyhow::Result<bool> {
    let hub_url = resolve_install_hub_url()?;
    let mut hub = HubClient::new(&hub_url)?;
    if let Ok(token) = std::env::var("ATLAS_TOKEN") {
        if !token.trim().is_empty() {
            hub.set_token(token);
        }
    }

    let arch = normalize_distribution_arch(std::env::consts::ARCH)?;
    let release = hub
        .get_latest_distribution_release("runner", "linux", arch)
        .await?;
    if !is_outdated_version(env!("ATLAS_BUILD_VERSION"), &release.version) {
        return Ok(false);
    }

    let asset = select_runner_asset(&release)?;
    let bytes = hub.download_distribution_asset(&asset.download_id).await?;
    write_binary_to_install_path(&PathBuf::from("/usr/local/bin/atlas-runner"), &bytes)?;
    Ok(true)
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
    write_binary_to_install_path(&install_path, &bytes)?;

    Ok(install_path)
}

fn select_runnerd_asset(
    release: &DistributionReleaseResponse,
) -> anyhow::Result<&DistributionReleaseAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.kind == "binary")
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.kind == "installer")
        })
        .ok_or_else(|| {
            anyhow::anyhow!("No runnerd binary/installer asset found in distribution release")
        })
}

fn select_runner_asset(
    release: &DistributionReleaseResponse,
) -> anyhow::Result<&DistributionReleaseAsset> {
    release
        .assets
        .iter()
        .find(|asset| asset.kind == "binary")
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.kind == "installer")
        })
        .ok_or_else(|| {
            anyhow::anyhow!("No runner binary/installer asset found in distribution release")
        })
}

fn normalize_version_for_compare(value: &str) -> String {
    value.trim().trim_start_matches('v').to_string()
}

fn is_outdated_version(current: &str, latest: &str) -> bool {
    let current_norm = normalize_version_for_compare(current);
    let latest_norm = normalize_version_for_compare(latest);
    match (Version::parse(&current_norm), Version::parse(&latest_norm)) {
        (Ok(current_semver), Ok(latest_semver)) => current_semver < latest_semver,
        _ => current_norm != latest_norm,
    }
}

fn write_binary_to_install_path(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temp_path = path.with_extension("tmp");
    std::fs::write(&temp_path, bytes).map_err(|err| {
        anyhow::anyhow!("Failed to write binary to {}: {err}", temp_path.display())
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755)).map_err(
            |err| {
                anyhow::anyhow!(
                    "Failed to set executable permissions on {}: {err}",
                    temp_path.display()
                )
            },
        )?;
    }

    std::fs::rename(&temp_path, path).map_err(|err| {
        anyhow::anyhow!(
            "Failed to move binary into place at {}: {err}",
            path.display()
        )
    })?;

    Ok(())
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

fn reconcile_runnerd_service_file(
    path: &Path,
    runnerd: &str,
    service_user: &str,
) -> anyhow::Result<()> {
    let contents = if path.exists() {
        std::fs::read_to_string(path)
            .map_err(|err| anyhow::anyhow!("Failed to read {}: {err}", path.display()))?
    } else {
        String::new()
    };

    let updated = reconcile_runnerd_service_content(&contents, runnerd, service_user);
    if normalize_newline(&contents) != normalize_newline(&updated) {
        std::fs::write(path, updated)
            .map_err(|err| anyhow::anyhow!("Failed to write {}: {err}", path.display()))?;
    }

    Ok(())
}

fn reconcile_runnerd_service_content(existing: &str, runnerd: &str, service_user: &str) -> String {
    if existing.trim().is_empty() {
        return format!(
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
Environment=RUST_LOG=info\n\
Environment=ATLAS_SYSTEMD_MANAGED=1\n\n\
[Install]\n\
WantedBy=multi-user.target\n"
        );
    }

    let lines = existing
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let Some(service_start) = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[Service]"))
    else {
        return format!(
            "{}\n\n[Service]\nType=simple\nUser={service_user}\nExecStart={runnerd}\nRestart=always\nRestartSec=5\nEnvironment=RUST_LOG=info\nEnvironment=ATLAS_SYSTEMD_MANAGED=1\n",
            normalize_newline(existing).trim_end()
        );
    };

    let service_end = lines
        .iter()
        .enumerate()
        .skip(service_start + 1)
        .find_map(|(idx, line)| {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                Some(idx)
            } else {
                None
            }
        })
        .unwrap_or(lines.len());

    let mut has_user = false;
    let mut has_type = false;
    let mut kept_service_lines = Vec::new();
    for line in &lines[service_start + 1..service_end] {
        if let Some((key, value)) = parse_service_key_value(line) {
            if key.eq_ignore_ascii_case("User") {
                has_user = true;
            }
            if key.eq_ignore_ascii_case("Type") && value.eq_ignore_ascii_case("simple") {
                has_type = true;
            }
            if is_managed_service_line(key, value) {
                continue;
            }
        }
        kept_service_lines.push(line.clone());
    }

    if !has_type {
        kept_service_lines.push("Type=simple".to_string());
    }
    if !has_user {
        kept_service_lines.push(format!("User={service_user}"));
    }
    kept_service_lines.push(format!("ExecStart={runnerd}"));
    kept_service_lines.push("Restart=always".to_string());
    kept_service_lines.push("RestartSec=5".to_string());
    kept_service_lines.push("Environment=RUST_LOG=info".to_string());
    kept_service_lines.push("Environment=ATLAS_SYSTEMD_MANAGED=1".to_string());

    let mut merged = Vec::new();
    merged.extend(lines[..service_start + 1].iter().cloned());
    merged.extend(kept_service_lines);
    merged.extend(lines[service_end..].iter().cloned());

    normalize_newline(&merged.join("\n"))
}

fn parse_service_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return None;
    }
    let (key, value) = trimmed.split_once('=')?;
    Some((key.trim(), value.trim()))
}

fn is_managed_service_line(key: &str, value: &str) -> bool {
    if key.eq_ignore_ascii_case("ExecStart")
        || key.eq_ignore_ascii_case("Restart")
        || key.eq_ignore_ascii_case("RestartSec")
    {
        return true;
    }

    if key.eq_ignore_ascii_case("Environment") {
        let normalized = value.trim().trim_matches('"').trim_matches('\'').trim();
        return normalized.starts_with("RUST_LOG=")
            || normalized.starts_with("ATLAS_SYSTEMD_MANAGED=");
    }

    false
}

fn normalize_newline(value: &str) -> String {
    let mut output = value.to_string();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn current_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| "atlas".to_string())
}
