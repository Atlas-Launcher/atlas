use anyhow::{Result, bail, Context};
use crate::hub::{HubClient, whitelist::WhitelistSync};
use atlas_client::sse::SseParser;
use crate::reconcile::Reconciler;
use crate::supervisor::Supervisor;
use crate::hub::whitelist::InstanceConfig;
use crate::rcon::{load_rcon_settings, RconClient};
use futures::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::process::Command as StdCommand;
use std::process::Stdio;
use tokio::sync::mpsc;
use tokio::time::{interval, sleep, Duration};
use tokio::fs;
use crate::java::ensure_java_for_minecraft;
use protocol::config::atlas::parse_config;
use crate::backup;
use chrono::{Local, NaiveTime, TimeZone};
use rand::RngCore;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackUpdateEvent {
    pack_id: String,
    #[serde(rename = "type")]
    event_type: Option<String>,
    channel: Option<String>,
    build_id: Option<String>,
}

async fn sync_whitelist_and_reload(
    whitelist: &WhitelistSync,
    pack_id: &str,
    runtime_dir: &PathBuf,
) {
    match whitelist.sync(pack_id).await {
        Ok(true) => {
            if let Ok(Some(settings)) = load_rcon_settings(runtime_dir).await {
                let rcon = RconClient::new(settings.address, settings.password);
                let _ = rcon.execute("whitelist reload").await;
            }
        }
        Ok(false) => {}
        Err(err) => println!("Whitelist sync failed: {err}"),
    }
}

async fn listen_pack_update_events(
    hub: Arc<HubClient>,
    pack_id: &str,
    channel: &str,
    updates: &mpsc::Sender<()>,
) -> Result<()> {
    let response = hub.open_pack_update_events(pack_id).await?;
    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for payload in parser.push_chunk(&chunk) {
            if should_trigger_pack_update(&payload, pack_id, channel) {
                let _ = updates.send(()).await;
            }
        }
    }

    bail!("Pack update stream ended")
}

fn should_trigger_pack_update(payload: &str, pack_id: &str, channel: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(event) = serde_json::from_str::<PackUpdateEvent>(payload) {
        let channel_matches = event
            .channel
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(channel))
            .unwrap_or(true);
        return event.pack_id == pack_id
            && channel_matches
            && event.event_type.as_deref() != Some("ready");
    }

    false
}

async fn run_pack_update_listener(
    hub: Arc<HubClient>,
    pack_id: String,
    channel: String,
    updates: mpsc::Sender<()>,
) {
    let mut backoff = Duration::from_secs(2);
    loop {
        if let Err(err) = listen_pack_update_events(hub.clone(), &pack_id, &channel, &updates).await {
            println!("Pack update stream error: {err}");
        }

        sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_secs(60));
    }
}

pub async fn exec(_force_config: bool, attach: bool, skip_setup: bool) -> Result<()> {
    if !skip_setup {
        run_setup(_force_config).await?;

        if !attach {
            spawn_background_after_setup()?;
            println!("Runner handed off to background. Logs: runtime/current/runner.log");
            return Ok(());
        }
    }

    run_server().await
}

async fn run_setup(_force_config: bool) -> Result<()> {

    ensure_server_stopped().await;

    let instance_path = PathBuf::from("instance.toml");
    let mut config = InstanceConfig::load(&instance_path).await
        .context("Missing instance.toml. Run `atlas-runner auth` first.")?;

    if config.memory.is_none() {
        config.memory = Some(crate::runner_config::default_memory()?);
        let _ = config.save(&instance_path).await;
    }
    
    let _hub = Arc::new(HubClient::new(&config.hub_url)?);
    let mut hub_mut = HubClient::new(&config.hub_url)?;
    if let Some(service_token) = config.service_token.clone() {
        hub_mut.set_service_token(service_token);
    } else if let Some(token) = config.token.clone() {
        hub_mut.set_token(token);
    } else {
        bail!("Missing auth token. Run `atlas-runner auth` first.");
    }
    let hub = Arc::new(hub_mut);

    let _cache_dir = PathBuf::from("cache");
    let _cache = Arc::new(crate::cache::Cache::new(_cache_dir));
    _cache.init().await?;

    let _fetcher = Arc::new(crate::fetch::Fetcher::new(_cache.clone()));
    
    // Reconcile
    let reconciler = Reconciler::new(hub.clone(), _fetcher.clone(), _cache.clone(), PathBuf::from("."));
    reconciler
        .reconcile(&config.pack_id, &config.channel)
        .await
        .context("Failed to pull build. Run `atlas-runner auth` to refresh credentials.")?;

    let mut config = InstanceConfig::load(&instance_path).await
        .context("Missing instance.toml after reconcile.")?;

    let runtime_dir = PathBuf::from("runtime/current");
    let java_bin = ensure_java_for_runtime().await?;
    ensure_server_files(&runtime_dir, &mut config, &instance_path, &java_bin).await?;
    ensure_eula(&runtime_dir).await?;
    ensure_server_properties(&runtime_dir, &config).await?;

    // Whitelist sync
    let whitelist = WhitelistSync::new(hub.clone(), runtime_dir.clone());
    sync_whitelist_and_reload(&whitelist, &config.pack_id, &runtime_dir).await;

    Ok(())
}

async fn run_server() -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let mut config = InstanceConfig::load(&instance_path).await
        .context("Missing instance.toml. Run `atlas-runner auth` first.")?;

    let mut hub_mut = HubClient::new(&config.hub_url)?;
    if let Some(service_token) = config.service_token.clone() {
        hub_mut.set_service_token(service_token);
    } else if let Some(token) = config.token.clone() {
        hub_mut.set_token(token);
    } else {
        bail!("Missing auth token. Run `atlas-runner auth` first.");
    }
    let hub = Arc::new(hub_mut);

    let runtime_dir = PathBuf::from("runtime/current");
    let whitelist = WhitelistSync::new(hub.clone(), runtime_dir.clone());
    sync_whitelist_and_reload(&whitelist, &config.pack_id, &runtime_dir).await;

    let java_bin = ensure_java_for_runtime().await?;
    ensure_server_files(&runtime_dir, &mut config, &instance_path, &java_bin).await?;
    ensure_eula(&runtime_dir).await?;
    ensure_server_properties(&runtime_dir, &config).await?;
    let max_ram = config.memory.clone().unwrap_or_else(|| "2G".to_string());
    let launch = resolve_launch_command(&runtime_dir, &max_ram, &java_bin).await?;
    let mut supervisor = Supervisor::new(
        runtime_dir.clone(),
        launch.command,
        launch.args,
        launch.envs,
    );

    ensure_server_stopped().await;
    println!("Setup complete. Launching Minecraft server...");
    let mut child = supervisor.spawn().await?;
    let mut restart_backoff = Duration::from_secs(2);

    let (update_tx, mut update_rx) = mpsc::channel::<()>(4);
    tokio::spawn(run_pack_update_listener(
        hub.clone(),
        config.pack_id.clone(),
        config.channel.clone(),
        update_tx,
    ));

    tokio::spawn(run_daily_backups(runtime_dir.clone()));

    let mut ticker = interval(Duration::from_secs(600));
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                sync_whitelist_and_reload(&whitelist, &config.pack_id, &runtime_dir).await;
            }
            update = update_rx.recv() => {
                if update.is_some() {
                    println!("Pack update detected. Restarting server...");
                    if let Ok(Some(settings)) = load_rcon_settings(&runtime_dir).await {
                        let rcon = RconClient::new(settings.address, settings.password);
                        let _ = rcon.execute("stop").await;
                    }
                    let _ = child.kill().await;
                    if let Err(err) = run_setup(false).await {
                        println!("Update failed: {err}");
                    }
                    let java_bin = ensure_java_for_runtime().await?;
                    let mut config = InstanceConfig::load(&instance_path).await
                        .context("Missing instance.toml. Run `atlas-runner auth` first.")?;
                    ensure_server_files(&runtime_dir, &mut config, &instance_path, &java_bin).await?;
                    ensure_eula(&runtime_dir).await?;
                    ensure_server_properties(&runtime_dir, &config).await?;
                    let max_ram = config.memory.clone().unwrap_or_else(|| "2G".to_string());
                    let launch = resolve_launch_command(&runtime_dir, &max_ram, &java_bin).await?;
                    supervisor = Supervisor::new(
                        runtime_dir.clone(),
                        launch.command,
                        launch.args,
                        launch.envs,
                    );
                    ensure_server_stopped().await;
                    child = supervisor.spawn().await?;
                    restart_backoff = Duration::from_secs(2);
                }
            }
            status = child.wait() => {
                println!("Server exited with status: {}", status?);
                println!("Restarting server after {:?}...", restart_backoff);
                sleep(restart_backoff).await;
                restart_backoff = (restart_backoff * 2).min(Duration::from_secs(60));
                let java_bin = ensure_java_for_runtime().await?;
                let mut config = InstanceConfig::load(&instance_path).await
                    .context("Missing instance.toml. Run `atlas-runner auth` first.")?;
                ensure_server_files(&runtime_dir, &mut config, &instance_path, &java_bin).await?;
                ensure_eula(&runtime_dir).await?;
                ensure_server_properties(&runtime_dir, &config).await?;
                let max_ram = config.memory.clone().unwrap_or_else(|| "2G".to_string());
                let launch = resolve_launch_command(&runtime_dir, &max_ram, &java_bin).await?;
                supervisor = Supervisor::new(
                    runtime_dir.clone(),
                    launch.command,
                    launch.args,
                    launch.envs,
                );
                ensure_server_stopped().await;
                child = supervisor.spawn().await?;
            }
        }
    }

}

fn spawn_background_after_setup() -> Result<()> {
    let current_exe = std::env::current_exe().context("Failed to resolve current executable")?;
    let runtime_dir = PathBuf::from("runtime/current");
    std::fs::create_dir_all(&runtime_dir).context("Failed to create runtime directory")?;

    let stdout_path = runtime_dir.join("runner.log");
    let stderr_path = runtime_dir.join("runner.err.log");
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stdout_path)
        .context("Failed to open runner.log")?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
        .context("Failed to open runner.err.log")?;

    let mut cmd = StdCommand::new(current_exe);
    cmd.arg("up");
    cmd.arg("--attach");
    cmd.arg("--skip-setup");

    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .current_dir(std::env::current_dir().context("Failed to resolve current directory")?);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let child = cmd.spawn().context("Failed to spawn background runner")?;
    let pid_file = runtime_dir.join("runner.pid");
    std::fs::write(&pid_file, child.id().to_string()).ok();

    println!("Runner started in background (pid {}).", child.id());
    Ok(())
}

async fn ensure_server_stopped() {
    let runtime_dir = PathBuf::from("runtime/current");
    if let Ok(Some(settings)) = load_rcon_settings(&runtime_dir).await {
        let rcon = RconClient::new(settings.address, settings.password);
        let _ = rcon.execute("stop").await;
    }

    let pid_file = PathBuf::from("runtime/current/server.pid");
    let pid = tokio::fs::read_to_string(&pid_file)
        .await
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    if let Some(pid) = pid {
        let _ = StdCommand::new("kill").arg(pid.to_string()).status();
        let _ = StdCommand::new("kill").arg("-0").arg(pid.to_string()).status().ok();
        let _ = StdCommand::new("kill").arg("-9").arg(pid.to_string()).status();
        let _ = tokio::fs::remove_file(&pid_file).await;
    }

    for pid in find_server_pids() {
        let _ = StdCommand::new("kill").arg(pid.to_string()).status();
        let _ = StdCommand::new("kill").arg("-9").arg(pid.to_string()).status();
    }
}

fn find_server_pids() -> Vec<u32> {
    let patterns = [
        "runtime/current",
        "server.jar",
        "fabric-server-launch.jar",
        "unix_args.txt",
    ];

    let mut pids = Vec::new();
    for pattern in patterns {
        if let Ok(output) = StdCommand::new("pgrep").arg("-f").arg(pattern).output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for line in text.lines() {
                    if let Ok(pid) = line.trim().parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }
    }

    pids.sort_unstable();
    pids.dedup();
    pids
}

async fn run_daily_backups(runtime_dir: PathBuf) {
    let archive_dir = runtime_dir.join("world-backup");
    loop {
        let now = Local::now();
        let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
        let next_midnight = Local
            .from_local_datetime(&tomorrow.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
            .single()
            .unwrap_or_else(|| now + chrono::Duration::days(1));
        let wait = (next_midnight - now)
            .to_std()
            .unwrap_or(Duration::from_secs(86400));

        sleep(wait).await;

        let _ = backup::archive_worlds(&runtime_dir, &archive_dir, "worlds", 5).await;
    }
}

async fn ensure_java_for_runtime() -> Result<String> {
    let instance_path = PathBuf::from("instance.toml");
    let config = InstanceConfig::load(&instance_path).await
        .context("Missing instance.toml. Run `atlas-runner auth` first.")?;
    let mc_version = config
        .minecraft_version
        .as_deref()
        .unwrap_or("1.20.1");

    let java_bin = ensure_java_for_minecraft(mc_version, config.java_major).await?;
    Ok(java_bin.to_string_lossy().to_string())
}

async fn ensure_server_files(
    runtime_dir: &PathBuf,
    config: &mut InstanceConfig,
    instance_path: &PathBuf,
    java_bin: &str,
) -> Result<()> {
    if has_server_launch_files(runtime_dir).await {
        return Ok(());
    }

    let cache_dir = PathBuf::from("cache/loader");
    fs::create_dir_all(&cache_dir).await?;

    if config.modloader_version.is_none() {
        if let Some((modloader, modloader_version)) = load_atlas_versions(runtime_dir).await {
            config.modloader = Some(modloader);
            config.modloader_version = Some(modloader_version);
        }
    }

    let mc_version = config
        .minecraft_version
        .as_deref()
        .context("Missing minecraft_version in instance.toml")?;
    let modloader = config
        .modloader
        .as_deref()
        .unwrap_or("fabric")
        .to_ascii_lowercase();

    match modloader.as_str() {
        "fabric" => {
            let loader_version = resolve_fabric_loader_version(mc_version, config.modloader_version.clone()).await?;
            let url = format!(
                "https://meta.fabricmc.net/v2/versions/loader/{mc_version}/{loader_version}/server/jar"
            );
            let cache_path = cache_dir.join(format!("fabric-server-{mc_version}-{loader_version}.jar"));
            download_to_path(&url, &cache_path).await?;
            let dest = runtime_dir.join("fabric-server-launch.jar");
            copy_if_missing(&cache_path, &dest).await?;
            config.modloader_version = Some(loader_version);
        }
        "forge" => {
            let loader_version = resolve_forge_loader_version(mc_version, config.modloader_version.clone()).await?;
            let installer_url = format!(
                "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc_version}-{loader_version}/forge-{mc_version}-{loader_version}-installer.jar"
            );
            let cache_path = cache_dir.join(format!("forge-installer-{mc_version}-{loader_version}.jar"));
            download_to_path(&installer_url, &cache_path).await?;
            let installer_path = runtime_dir.join("forge-installer.jar");
            copy_if_missing(&cache_path, &installer_path).await?;
            run_server_installer(java_bin, runtime_dir, &installer_path).await?;
            config.modloader_version = Some(loader_version);
        }
        "neoforge" | "neo" => {
            let loader_version = resolve_neoforge_loader_version(mc_version, config.modloader_version.clone()).await?;
            let installer_url = format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar"
            );
            let cache_path = cache_dir.join(format!("neoforge-installer-{loader_version}.jar"));
            download_to_path(&installer_url, &cache_path).await?;
            let installer_path = runtime_dir.join("neoforge-installer.jar");
            copy_if_missing(&cache_path, &installer_path).await?;
            run_server_installer(java_bin, runtime_dir, &installer_path).await?;
            config.modloader_version = Some(loader_version);
        }
        other => {
            bail!("Unsupported modloader: {}", other);
        }
    }

    config.save(instance_path).await?;

    if !has_server_launch_files(runtime_dir).await {
        bail!("Server installer did not produce launch files in runtime/current");
    }
    Ok(())
}

async fn has_server_launch_files(runtime_dir: &PathBuf) -> bool {
    runtime_dir.join("run.sh").exists()
        || runtime_dir.join("fabric-server-launch.jar").exists()
        || runtime_dir.join("server.jar").exists()
}

async fn download_to_path(url: &str, dest: &PathBuf) -> Result<()> {
    if let Ok(meta) = fs::metadata(dest).await {
        if meta.len() > 0 {
            return Ok(());
        }
    }
    let response = reqwest::get(url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    fs::write(dest, &bytes).await?;
    Ok(())
}

async fn copy_if_missing(source: &PathBuf, dest: &PathBuf) -> Result<()> {
    if let Ok(meta) = fs::metadata(dest).await {
        if meta.len() > 0 {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::copy(source, dest).await?;
    Ok(())
}

async fn ensure_eula(runtime_dir: &PathBuf) -> Result<()> {
    let eula_path = runtime_dir.join("eula.txt");
    let content = "# Generated by atlas-runner\neula=true\n";
    fs::write(&eula_path, content).await?;
    Ok(())
}

async fn ensure_server_properties(runtime_dir: &PathBuf, config: &InstanceConfig) -> Result<()> {
    let server_props = runtime_dir.join("server.properties");
    let template = runtime_dir.join("config/server.properties");
    if !server_props.exists() {
        if template.exists() {
            if let Some(parent) = server_props.parent() {
                fs::create_dir_all(parent).await?;
            }
            let _ = fs::copy(&template, &server_props).await;
        }
    }

    let mut current = fs::read_to_string(&server_props).await.unwrap_or_default();
    if current.trim().is_empty() {
        current = String::new();
    }

    let template_contents = if template.exists() {
        fs::read_to_string(&template).await.ok()
    } else {
        None
    };

    let template_rcon_password = template_contents
        .as_deref()
        .and_then(|contents| get_property(contents, "rcon.password"));
    let template_rcon_port = template_contents
        .as_deref()
        .and_then(|contents| get_property(contents, "rcon.port"));
    let template_rcon_enabled = template_contents
        .as_deref()
        .and_then(|contents| get_property(contents, "enable-rcon"));

    let default_port = config.port.unwrap_or(25565);
    let mut updated = update_server_port(&current, default_port);
    let rcon_enabled = template_rcon_enabled.as_deref().unwrap_or("true");
    updated = set_property(&updated, "enable-rcon", rcon_enabled);

    if let Some(port) = template_rcon_port.as_deref() {
        updated = set_property(&updated, "rcon.port", port);
    } else {
        updated = set_property(&updated, "rcon.port", "25575");
    }

    if let Some(password) = template_rcon_password.as_deref() {
        updated = set_property(&updated, "rcon.password", password);
    } else if !has_property(&updated, "rcon.password") {
        let password = generate_rcon_password();
        updated = set_property(&updated, "rcon.password", &password);
    }

    fs::write(&server_props, updated).await?;

    Ok(())
}

fn update_server_port(contents: &str, port: u16) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in contents.lines() {
        if line.trim_start().starts_with("server-port=") {
            lines.push(format!("server-port={}", port));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }

    if !replaced {
        lines.push(format!("server-port={}", port));
    }

    format!("{}\n", lines.join("\n"))
}

fn has_property(contents: &str, key: &str) -> bool {
    contents.lines().any(|line| line.trim_start().starts_with(&format!("{}=", key)))
}

fn get_property(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((current_key, value)) = trimmed.split_once('=') else { continue };
        if current_key.trim() == key {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            return Some(value.to_string());
        }
    }
    None
}

fn ensure_property(contents: &str, key: &str, value: &str) -> String {
    if has_property(contents, key) {
        return contents.to_string();
    }
    let mut lines = Vec::new();
    lines.extend(contents.lines().map(|line| line.to_string()));
    lines.push(format!("{}={}", key, value));
    format!("{}\n", lines.join("\n"))
}

fn set_property(contents: &str, key: &str, value: &str) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    for line in contents.lines() {
        if line.trim_start().starts_with(&format!("{}=", key)) {
            lines.push(format!("{}={}", key, value));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("{}={}", key, value));
    }
    format!("{}\n", lines.join("\n"))
}

fn generate_rcon_password() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn load_atlas_versions(runtime_dir: &PathBuf) -> Option<(String, String)> {
    let atlas_path = runtime_dir.join("atlas.toml");
    let contents = fs::read_to_string(&atlas_path).await.ok()?;
    let atlas_config = parse_config(&contents).ok()?;
    Some((
        atlas_config.versions.modloader.to_ascii_lowercase(),
        atlas_config.versions.modloader_version,
    ))
}

async fn run_server_installer(java_bin: &str, runtime_dir: &PathBuf, installer: &PathBuf) -> Result<()> {
    let installer_arg = installer
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| installer.to_string_lossy().to_string());
    let status = tokio::process::Command::new(java_bin)
        .current_dir(runtime_dir)
        .arg("-jar")
        .arg(installer_arg)
        .arg("--installServer")
        .status()
        .await?;

    if !status.success() {
        bail!("Server installer failed with exit code: {}", status);
    }
    Ok(())
}

async fn resolve_fabric_loader_version(
    mc_version: &str,
    requested: Option<String>,
) -> Result<String> {
    if let Some(value) = requested.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Ok(value);
        }
    }

    #[derive(Deserialize)]
    struct FabricEntry {
        loader: FabricLoaderInfo,
    }

    #[derive(Deserialize)]
    struct FabricLoaderInfo {
        version: String,
        stable: Option<bool>,
    }

    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc_version}");
    let entries: Vec<FabricEntry> = reqwest::get(&url).await?.error_for_status()?.json().await?;
    let chosen = entries
        .iter()
        .find(|entry| entry.loader.stable.unwrap_or(false))
        .or_else(|| entries.first())
        .context("No Fabric loader versions found")?;
    Ok(chosen.loader.version.clone())
}

async fn resolve_forge_loader_version(
    mc_version: &str,
    requested: Option<String>,
) -> Result<String> {
    if let Some(value) = requested.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Ok(value);
        }
    }

    let metadata = reqwest::get("https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml")
        .await?
        .error_for_status()?
        .text()
        .await?;
    let prefix = format!("{}-", mc_version);
    let mut versions = extract_versions_from_maven_metadata(&metadata)
        .into_iter()
        .filter_map(|entry| entry.strip_prefix(&prefix).map(|value| value.to_string()))
        .collect::<Vec<_>>();
    sort_versions_desc(&mut versions);
    versions
        .first()
        .cloned()
        .context("No Forge versions found for Minecraft version")
}

async fn resolve_neoforge_loader_version(
    mc_version: &str,
    requested: Option<String>,
) -> Result<String> {
    if let Some(value) = requested.map(|value| value.trim().to_string()) {
        if !value.is_empty() {
            return Ok(value);
        }
    }

    let metadata = reqwest::get("https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml")
        .await?
        .error_for_status()?
        .text()
        .await?;
    let all_versions = extract_versions_from_maven_metadata(&metadata);
    let candidate_lines = neoforge_candidate_lines(mc_version)?;

    let mut versions = Vec::new();
    for line in candidate_lines {
        let exact_prefix = format!("{}.", line);
        versions = all_versions
            .iter()
            .filter(|value| value.starts_with(&exact_prefix))
            .cloned()
            .collect::<Vec<_>>();

        if versions.is_empty() {
            let fallback_prefix = format!("{}-", line);
            versions = all_versions
                .iter()
                .filter(|value| value.starts_with(&fallback_prefix))
                .cloned()
                .collect::<Vec<_>>();
        }

        if !versions.is_empty() {
            break;
        }
    }

    sort_versions_desc(&mut versions);
    versions
        .first()
        .cloned()
        .context("No NeoForge versions found for Minecraft version")
}

fn extract_versions_from_maven_metadata(metadata: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = metadata;

    loop {
        let Some(start_index) = rest.find("<version>") else {
            break;
        };
        let after_start = &rest[start_index + "<version>".len()..];
        let Some(end_index) = after_start.find("</version>") else {
            break;
        };

        let version = after_start[..end_index].trim();
        if !version.is_empty() {
            versions.push(version.to_string());
        }

        rest = &after_start[end_index + "</version>".len()..];
    }

    versions
}

fn neoforge_candidate_lines(mc_version: &str) -> Result<Vec<String>> {
    let parts = mc_version.split('.').collect::<Vec<_>>();
    if parts.len() < 3 || parts[0] != "1" {
        bail!("Unsupported Minecraft version format for NeoForge: {}", mc_version);
    }

    let major = parts[1].parse::<u64>()?;
    let patch = parts[2].parse::<u64>()?;

    if major == 20 && patch == 1 {
        return Ok(vec!["20.2".to_string(), "20.1".to_string()]);
    }

    Ok(vec![format!("{}.{}", major, patch)])
}

fn sort_versions_desc(versions: &mut [String]) {
    versions.sort_by(|left, right| compare_version_like(right, left));
}

fn compare_version_like(left: &str, right: &str) -> std::cmp::Ordering {
    let left_tokens = tokenize_version(left);
    let right_tokens = tokenize_version(right);
    let max = left_tokens.len().max(right_tokens.len());

    for index in 0..max {
        let left_token = left_tokens.get(index);
        let right_token = right_tokens.get(index);
        match (left_token, right_token) {
            (Some(VersionToken::Number(left_num)), Some(VersionToken::Number(right_num))) => {
                let order = left_num.cmp(right_num);
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(VersionToken::Text(left_text)), Some(VersionToken::Text(right_text))) => {
                let order = left_text.cmp(right_text);
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
            (Some(VersionToken::Number(_)), Some(VersionToken::Text(_))) => {
                return std::cmp::Ordering::Greater;
            }
            (Some(VersionToken::Text(_)), Some(VersionToken::Number(_))) => {
                return std::cmp::Ordering::Less;
            }
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (None, None) => break,
        }
    }

    left.cmp(right)
}

#[derive(Debug)]
enum VersionToken {
    Number(u64),
    Text(String),
}

fn tokenize_version(value: &str) -> Vec<VersionToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_number: Option<bool> = None;

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            if current_is_number == Some(true) {
                current.push(ch);
            } else {
                push_token(&mut tokens, &mut current, current_is_number);
                current.push(ch);
                current_is_number = Some(true);
            }
            continue;
        }

        if ch.is_ascii_alphabetic() {
            let lower = ch.to_ascii_lowercase();
            if current_is_number == Some(false) {
                current.push(lower);
            } else {
                push_token(&mut tokens, &mut current, current_is_number);
                current.push(lower);
                current_is_number = Some(false);
            }
            continue;
        }

        push_token(&mut tokens, &mut current, current_is_number);
        current_is_number = None;
    }

    push_token(&mut tokens, &mut current, current_is_number);
    tokens
}

fn push_token(tokens: &mut Vec<VersionToken>, current: &mut String, current_is_number: Option<bool>) {
    if current.is_empty() {
        return;
    }

    if current_is_number == Some(true) {
        let value = current.parse::<u64>().unwrap_or(0);
        tokens.push(VersionToken::Number(value));
    } else {
        tokens.push(VersionToken::Text(current.clone()));
    }

    current.clear();
}

struct LaunchCommand {
    command: String,
    args: Vec<String>,
    envs: Vec<(String, String)>,
}

async fn resolve_launch_command(
    runtime_dir: &PathBuf,
    max_ram: &str,
    java_bin: &str,
) -> Result<LaunchCommand> {
    let unix_args = runtime_dir.join("unix_args.txt");
    if unix_args.exists() {
        write_user_jvm_args(runtime_dir, max_ram).await?;
        return Ok(LaunchCommand {
            command: java_bin.to_string(),
            args: vec!["@user_jvm_args.txt".to_string(), "@unix_args.txt".to_string()],
            envs: Vec::new(),
        });
    }

    let run_sh = runtime_dir.join("run.sh");
    if run_sh.exists() {
        write_user_jvm_args(runtime_dir, max_ram).await?;
        let envs = build_java_env(java_bin);
        return Ok(LaunchCommand {
            command: "sh".to_string(),
            args: vec!["run.sh".to_string()],
            envs,
        });
    }

    let fabric_launch = runtime_dir.join("fabric-server-launch.jar");
    if fabric_launch.exists() {
        return Ok(LaunchCommand {
            command: java_bin.to_string(),
            args: vec![
                format!("-Xmx{}", max_ram),
                "-jar".to_string(),
                "fabric-server-launch.jar".to_string(),
                "nogui".to_string(),
            ],
            envs: Vec::new(),
        });
    }

    let server_jar = runtime_dir.join("server.jar");
    if server_jar.exists() {
        return Ok(LaunchCommand {
            command: java_bin.to_string(),
            args: vec![
                format!("-Xmx{}", max_ram),
                "-jar".to_string(),
                "server.jar".to_string(),
                "nogui".to_string(),
            ],
            envs: Vec::new(),
        });
    }

    bail!("Missing server launch files. Expected run.sh, fabric-server-launch.jar, or server.jar in runtime/current.")
}

fn build_java_env(java_bin: &str) -> Vec<(String, String)> {
    let mut envs = Vec::new();
    let java_path = PathBuf::from(java_bin);
    if let Some(bin_dir) = java_path.parent() {
        if let Some(java_home) = bin_dir.parent() {
            envs.push((
                "JAVA_HOME".to_string(),
                java_home.to_string_lossy().to_string(),
            ));
        }
        if let Ok(path) = std::env::var("PATH") {
            let updated = format!("{}:{}", bin_dir.to_string_lossy(), path);
            envs.push(("PATH".to_string(), updated));
        }
    }
    envs
}

async fn write_user_jvm_args(runtime_dir: &PathBuf, max_ram: &str) -> Result<()> {
    let args_path = runtime_dir.join("user_jvm_args.txt");
    let mut lines = Vec::new();
    if let Ok(content) = fs::read_to_string(&args_path).await {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("-Xmx") || trimmed.starts_with("-Xms") {
                continue;
            }
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }
    lines.push(format!("-Xmx{}", max_ram));
    let content = lines.join("\n");
    fs::write(&args_path, format!("{}\n", content)).await?;
    Ok(())
}
