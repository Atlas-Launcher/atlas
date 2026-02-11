pub mod backup;
mod commands;

use runner_v2_utils::{ensure_dir, runtime_paths_v2};
use std::time::Duration;
use tokio::process::Command;

pub use commands::auth::exec as auth;
pub use commands::core::{ping, shutdown, up};
pub use commands::rcon::{rcon_exec, rcon_interactive};
pub use commands::supervisor::{
    daemon_logs_tail, daemon_logs_tail_follow, logs_tail, logs_tail_follow, stop,
};

pub(crate) async fn connect_or_start() -> anyhow::Result<runner_ipc_v2::framing::FramedStream> {
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

pub(crate) async fn connect_only() -> anyhow::Result<runner_ipc_v2::framing::FramedStream> {
    let paths = runtime_paths_v2();
    runner_ipc_v2::socket::connect(&paths.socket_path)
        .await
        .map(runner_ipc_v2::framing::framed)
        .map_err(Into::into)
}

async fn start_daemon_detached() -> anyhow::Result<()> {
    // 1) Dev: run an arbitrary command via shell
    // Example:
    //   ATLAS_RUNNERD_CMD='cargo run -p runnerd-v2' cargo run -p runner-v2 -- ping
    if let Ok(cmd) = std::env::var("ATLAS_RUNNERD_CMD") {
        #[cfg(target_os = "macos")]
        let mut c = Command::new("sh");
        #[cfg(target_os = "linux")]
        let mut c = Command::new("sh");

        c.arg("-lc").arg(cmd);
        c.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(());
    }

    // 2) Dev/prod: explicit binary path
    // Example:
    //   ATLAS_RUNNERD_PATH=target/debug/runnerd2 cargo run -p runner-v2 -- ping
    if let Ok(path) = std::env::var("ATLAS_RUNNERD_PATH") {
        Command::new(path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(());
    }

    // 3) Default: hope atlas-runnerd is on PATH
    Command::new("atlas-runnerd")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    Ok(())
}
