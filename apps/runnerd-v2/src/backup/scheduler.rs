use chrono::{Local, NaiveDate, Duration as ChronoDuration};
use tokio::time::{sleep, Duration};
use tracing::{info, debug, warn};

use crate::supervisor::SharedState;

use super::ops;
use tokio::fs as async_fs;
use std::path::PathBuf;

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

    loop {
        let now = Local::now();
        let today = now.date_naive();

        // If we haven't run a backup for today, run it now. This covers the case where
        // the process was sleeping across midnight and woke up after midnight.
        if last_backup_date.as_ref() != Some(&today) {
            // Spawn the actual backup so IO doesn't block the scheduler loop
            let root = server_root.clone();
            let st = state.clone();
            info!("daily backup: starting backup for date {}", today);
            // Spawn and persist the last-run date when the backup completes successfully.
            tokio::spawn(async move {
                match ops::backup_world(&root, st).await {
                    Ok(path) => {
                        info!("daily backup completed: {}", path.display());
                        write_last_backup_date(&root, today).await;
                    }
                    Err(err) => {
                        warn!("daily backup failed: {}", err);
                    }
                }
            });
            // Wait a short while for the task to at least be scheduled and avoid immediate re-check.
            // We don't .await the handle (so backup runs in background), but ensure last_backup_date
            // updates to avoid re-triggering.
            last_backup_date = Some(today);
            // Small debounce to avoid tight loop
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
