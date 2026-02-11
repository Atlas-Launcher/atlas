use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};
use chrono::{Local, Timelike};
use tracing::info;

use crate::supervisor::SharedState;

use super::ops;

/// Runs a background task that triggers a backup once daily at midnight local time.
/// The task is non-blocking and uses sleeps between checks. It expects to be spawned
/// as a detached tokio task.
pub async fn run_daily_backup(server_root: std::path::PathBuf, state: SharedState) {
    // Compute the initial delay until next midnight
    loop {
        let now = Local::now();
        let tomorrow = now.date_naive().succ_opt().unwrap_or(now.date_naive());
        let next_midnight = chrono::DateTime::<Local>::from_local(tomorrow.and_hms_opt(0,0,0).unwrap(), Local);
        let dur = next_midnight.signed_duration_since(now).to_std().unwrap_or(Duration::from_secs(60));
        info!("daily backup scheduler sleeping for {}s", dur.as_secs());
        sleep(dur).await;

        // spawn backup in background (don't block the scheduler loop)
        let root = server_root.clone();
        let st = state.clone();
        tokio::spawn(async move {
            let _ = ops::backup_world(&root, st).await;
        });

        // sleep a minute to avoid double-triggering around DST boundary
        sleep(Duration::from_secs(60)).await;
    }
}

// Compatibility shim: re-export the scheduler's run_daily_backup function
pub use crate::backup::scheduler::run_daily_backup;
