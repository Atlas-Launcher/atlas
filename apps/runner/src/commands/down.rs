use crate::hub::whitelist::InstanceConfig;
use crate::rcon::{RconClient, load_rcon_settings};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use tokio::time::{Duration, sleep};

pub async fn exec() -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let _config = InstanceConfig::load(&instance_path)
        .await
        .context("No instance.toml found in current directory")?;

    let runner_pid_file = PathBuf::from("runtime/current/runner.pid");
    let mut stopped_any = false;
    if let Some(pid) = read_pid(&runner_pid_file).await {
        println!("Stopping runner process (pid {pid})...");
        if kill_pid(pid) {
            let _ = fs::remove_file(&runner_pid_file).await;
            stopped_any = true;
        }
    }

    let runtime_dir = PathBuf::from("runtime/current");
    let _ = try_rcon_stop(&runtime_dir).await;
    let mut waited_for_shutdown = false;

    let pid_file = runtime_dir.join("server.pid");
    if let Some(pid) = read_pid(&pid_file).await {
        println!("Stopping Minecraft server (pid {pid})...");
        println!("Waiting 30 seconds for graceful shutdown...");
        sleep(Duration::from_secs(30)).await;
        waited_for_shutdown = true;
        let still_running = read_pid(&pid_file).await == Some(pid);
        if !still_running {
            println!("Server stopped.");
            let _ = fs::remove_file(&pid_file).await;
            stopped_any = true;
        } else {
            if kill_pid(pid) {
                println!("Server stopped.");
                let _ = fs::remove_file(&pid_file).await;
                stopped_any = true;
            } else {
                let _ = fs::remove_file(&pid_file).await;
            }
        }
    }

    let fallback_pids = find_server_pids();
    if !fallback_pids.is_empty() {
        if !waited_for_shutdown {
            println!("Waiting 30 seconds for graceful shutdown...");
            sleep(Duration::from_secs(30)).await;
        }
        for pid in fallback_pids {
            println!("Stopping Minecraft server (pid {pid})...");
            if kill_pid(pid) {
                stopped_any = true;
            }
        }
    }

    if !stopped_any {
        println!("Server is not running.");
    }

    Ok(())
}

async fn read_pid(path: &PathBuf) -> Option<u32> {
    let pid_str = fs::read_to_string(path).await.ok()?;
    pid_str.trim().parse::<u32>().ok()
}

fn kill_pid(pid: u32) -> bool {
    Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
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
        if let Ok(output) = Command::new("pgrep").arg("-f").arg(pattern).output() {
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

async fn try_rcon_stop(runtime_dir: &PathBuf) -> Result<()> {
    if let Ok(Some(settings)) = load_rcon_settings(runtime_dir).await {
        let rcon = RconClient::new(settings.address, settings.password);
        let _ = rcon.execute("stop").await;
    }
    Ok(())
}
