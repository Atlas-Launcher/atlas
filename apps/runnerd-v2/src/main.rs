use tracing::{info, warn};

use runner_v2_utils::{ensure_dir, runtime_paths_v2};

mod lock;
mod daemon;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let paths = runtime_paths_v2();
    ensure_dir(&paths.runtime_dir)?;

    // single-instance lock
    let _guard = lock::acquire_lock(&paths.lock_path)?;

    // if a socket file exists, see if a daemon is alive
    if paths.socket_path.exists() {
        if runner_ipc_v2::socket::socket_alive(&paths.socket_path).await {
            warn!("daemon already running (socket alive), exiting");
            return Ok(());
        }
        // stale socket file
        runner_ipc_v2::socket::remove_stale_socket(&paths.socket_path)?;
    }

    let listener = runner_ipc_v2::socket::bind(&paths.socket_path).await?;
    info!("runnerd2 listening at {:?}", paths.socket_path);

    daemon::serve(listener).await
}
