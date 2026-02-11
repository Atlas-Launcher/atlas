pub mod ops;
pub mod rcon;
pub mod scheduler;

use std::path::PathBuf;
use crate::supervisor::SharedState;

/// Perform a backup before an update. This will use RCON save-off/save-all when available
/// to ensure a consistent copy of world directories. The function returns the created backup
/// directory path on success.
pub async fn backup_before_update(server_root: &PathBuf, state: SharedState) -> Result<PathBuf, String> {
    // Delegate to ops which will use rcon helper if possible
    ops::backup_world(server_root, state).await
}

/// Archive/move the current dir to a timestamped backup. Returns the backup path.
pub async fn move_current_to_backup(server_root: &PathBuf) -> Result<PathBuf, String> {
    ops::archive_current_for_force_reinstall(server_root).await
}

/// Start the daily backup scheduler (spawns a background task that runs at midnight).
pub fn start_daily_scheduler(server_root: PathBuf, state: SharedState) {
    tokio::spawn(async move {
        scheduler::run_daily_backup(server_root, state).await;
    });
}
