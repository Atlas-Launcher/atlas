use chrono::{Local, NaiveDate, Duration as ChronoDuration};
use tokio::time::{sleep, Duration};
use tracing::{info, debug, warn};

use crate::supervisor::SharedState;

use super::ops;
use tokio::fs as async_fs;
use std::path::PathBuf;
use tokio::task;
use std::fs as stdfs;
use std::time::{SystemTime, Duration as StdDuration};

/// Remove backups older than `keep_days` from server_root/.runner/backup using a blocking task.
async fn prune_old_backups(server_root: &PathBuf, keep_days: u64) {
    let backup_dir = server_root.join(".runner").join("backup");
    let dir = backup_dir.clone();
    let keep_secs = keep_days.saturating_mul(24 * 3600);
    let result = task::spawn_blocking(move || -> Result<(), String> {
        if !dir.exists() {
            return Ok(());
        }
        let cutoff = SystemTime::now()
            .checked_sub(StdDuration::from_secs(keep_secs))
            .ok_or_else(|| "time error computing cutoff".to_string())?;

        for entry in stdfs::read_dir(&dir).map_err(|e| format!("read_dir failed: {}", e))? {
            let entry = entry.map_err(|e| format!("read_dir entry failed: {}", e))?;
            let path = entry.path();
            // Use metadata modified time to decide
            match entry.metadata() {
                Ok(meta) => {
                    if let Ok(mtime) = meta.modified() {
                        if mtime < cutoff {
                            if meta.is_dir() {
                                if let Err(e) = stdfs::remove_dir_all(&path) {
                                    eprintln!("prune: failed to remove dir {}: {}", path.display(), e);
                                }
                            } else {
                                if let Err(e) = stdfs::remove_file(&path) {
                                    eprintln!("prune: failed to remove file {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("prune: metadata failed for {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }).await;

    if let Err(join_err) = result {
        warn!("prune operation join failed: {}", join_err);
    } else if let Ok(Err(err)) = result {
        warn!("prune operation failed: {}", err);
    }
}

async fn read_last_backup_date(server_root: &PathBuf) -> Option<NaiveDate> {
    let path = server_root.join(".runner").join("backup").join("last_backup.txt");
    match async_fs::read_to_string(&path).await {
        Ok(s) => {
            let s = s.trim();
            if s.is_empty() {
                return None;
            }
            if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                Some(d)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

async fn write_last_backup_date(server_root: &PathBuf, date: NaiveDate) {
    let dir = server_root.join(".runner").join("backup");
    let _ = async_fs::create_dir_all(&dir).await;
    let path = dir.join("last_backup.txt");
    let tmp = dir.join("last_backup.txt.tmp");
    let content = date.format("%Y-%m-%d").to_string();
    if let Err(e) = async_fs::write(&tmp, content.as_bytes()).await {
        warn!("failed to write last_backup tmp file: {}", e);
        return;
    }
    if let Err(e) = async_fs::rename(&tmp, &path).await {
        warn!("failed to persist last_backup file: {}", e);
        let _ = async_fs::remove_file(&tmp).await;
    }
}

/// Runs a background task that triggers a backup once daily at midnight local time.
/// The scheduler avoids noisy logs and ensures a backup is triggered immediately if
/// the process wakes after midnight (so we don't miss a backup when suspended).
pub async fn run_daily_backup(server_root: std::path::PathBuf, state: SharedState) {
    let mut last_backup_date: Option<NaiveDate> = read_last_backup_date(&server_root).await;

    // Kick off an initial prune in background (non-blocking). Use 14 days retention.
    let root_for_prune = server_root.clone();
    tokio::spawn(async move {
        prune_old_backups(&root_for_prune, 14).await;
    });

    loop {
        let now = Local::now();
        let today = now.date_naive();

        // If we haven't run a backup for today, run it now. This covers the case where
        // the process was sleeping across midnight and woke up after midnight.
        if last_backup_date.as_ref() != Some(&today) {
            // Run the backup and wait for completion so we only persist last-run on success.
            let root = server_root.clone();
            let st = state.clone();
            info!("daily backup: starting backup for date {}", today);
            match ops::backup_world(&root, st).await {
                Ok(path) => {
                    info!("daily backup completed: {}", path.display());
                    write_last_backup_date(&root, today).await;
                    prune_old_backups(&root, 14).await;
                    last_backup_date = Some(today);
                }
                Err(err) => {
                    warn!("daily backup failed: {}", err);
                    // Do not mark last_backup_date so we'll retry later (or on next wake)
                }
            }
            // Small debounce to avoid tight loop in failure cases
            sleep(Duration::from_secs(5)).await;
        }

        // Compute how long until next midnight and sleep. Use debug-level logging to avoid clutter.
        let next_midnight = (today + ChronoDuration::days(1)).and_hms_opt(0, 0, 0).unwrap();
        let dur = next_midnight - now.naive_local();
        let seconds = dur.num_seconds().max(60) as u64;
        debug!("daily backup scheduler sleeping for {}s until next midnight", seconds);
        sleep(Duration::from_secs(seconds)).await;
    }
}
