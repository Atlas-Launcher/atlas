use std::process::Command;
use tracing::{info, warn};

use runner_v2_utils::{ensure_dir, runtime_paths_v2};

mod backup;
mod config;
mod daemon;
mod lock;
mod self_update;
mod supervisor;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let logs = supervisor::LogStore::new(2000);
    let log_writer = logs.daemon_writer();
    tracing_subscriber::fmt().with_writer(log_writer).init();

    let paths = runtime_paths_v2();
    ensure_dir(&paths.runtime_dir)?;

    // single-instance lock
    let _guard = match lock::acquire_lock(&paths.lock_path) {
        Ok(guard) => guard,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            warn!("daemon already running (lock held), exiting");
            return Ok(());
        }
        Err(e) => return Err(e),
    };

    // if a socket file exists, see if a daemon is alive
    if paths.socket_path.exists() {
        if runner_ipc_v2::socket::socket_alive(&paths.socket_path).await {
            warn!("daemon already running (socket alive), exiting");
            return Ok(());
        }
        // stale socket file
        runner_ipc_v2::socket::remove_stale_socket(&paths.socket_path)?;
    }

    // If a Minecraft server process is already running on this host, exit with an obvious log
    if let Some((pid, cmdline)) = detect_existing_minecraft_process() {
        warn!(
            "detected existing Minecraft process (pid={}): {}. Exiting daemon to avoid conflicts.",
            pid, cmdline
        );
        return Ok(());
    }

    fn detect_existing_minecraft_process() -> Option<(i32, String)> {
        // Use `ps` to list processes and look for common MC server markers in the command line.
        // This avoids depending on sysinfo API version differences and keeps the check simple.
        let output = Command::new("ps")
            .args(["-axo", "pid,comm,args"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let candidate_substrs = [
            "minecraft",
            "minecraft_server",
            "server.jar",
            "paper.jar",
            "spigot.jar",
            "fabric-server-launch.jar",
            "vanilla.jar",
        ];

        for line in stdout.lines().skip(1) {
            let trimmed = line.trim();
            let mut parts = trimmed.split_whitespace();
            if let Some(pid_str) = parts.next() {
                if let Ok(pid) = pid_str.parse::<i32>() {
                    let rest = parts.collect::<Vec<_>>().join(" ");
                    let lc = rest.to_ascii_lowercase();
                    for sub in &candidate_substrs {
                        if lc.contains(sub) {
                            return Some((pid, rest));
                        }
                    }
                }
            }
        }
        None
    }

    let listener = runner_ipc_v2::socket::bind(&paths.socket_path).await?;
    info!("runnerd2 listening at {:?}", paths.socket_path);

    // auto-start is handled inside `daemon::serve`; just start serving now.

    daemon::serve(listener, logs).await
}
