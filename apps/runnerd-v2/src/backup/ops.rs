use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use std::fs as stdfs;
use tokio::task;
use tracing::{info, warn};

use crate::backup::rcon::{rcon_save_off, rcon_save_on};
use crate::supervisor::{SharedState, now_millis};

/// Perform a world backup while the server is running if possible.
/// Uses RCON save-off/save-on if available, and does the heavy I/O on a blocking thread.
pub async fn backup_world(server_root: &Path, _state: SharedState) -> Result<PathBuf, String> {
    let current = server_root.join("current");
    if !current.exists() {
        return Err("current directory does not exist".to_string());
    }

    // Try to flush and disable saves via RCON if possible
    let mut used_rcon = false;
    match rcon_save_off(server_root).await {
        Ok(true) => used_rcon = true,
        Ok(false) => {}
        Err(e) => warn!("rcon save-off failed: {}", e),
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("time error: {}", e))?
        .as_millis();
    let backup = server_root
        .join(".runner")
        .join("backup")
        .join(format!("backup-{}", ts));

    // Perform blocking copy on threadpool
    let cur = current.clone();
    let dst = backup.clone();
    // Use a temp directory while copying, then atomically rename into place.
    let tmp = dst.with_extension("tmp").clone();
    let copy_res: Result<(), String> = task::spawn_blocking(move || -> Result<(), String> {
        // Ensure parent exists
        if let Some(parent) = tmp.parent() {
            stdfs::create_dir_all(parent)
                .map_err(|e| format!("create backup parent dir failed: {}", e))?;
        }
        // Remove any stale tmp dir
        let _ = stdfs::remove_dir_all(&tmp);
        stdfs::create_dir_all(&tmp).map_err(|e| format!("create tmp backup dir failed: {}", e))?;

        let mut copied_entries: Vec<String> = Vec::new();

        // copy worlds
        for name in &["world", "world_nether", "world_the_end"] {
            let s = cur.join(name);
            if s.exists() {
                let d = tmp.join(name);
                copy_dir_recursive_blocking(&s, &d)?;
                copied_entries.push(name.to_string());
            }
        }

        // copy identity files
        for name in &[
            "whitelist.json",
            "ops.json",
            "banned-ips.json",
            "banned-players.json",
            "usercache.json",
        ] {
            let s = cur.join(name);
            if s.exists() {
                let d = tmp.join(name);
                stdfs::copy(&s, &d).map_err(|e| format!("copy file failed: {}", e))?;
                copied_entries.push(name.to_string());
            }
        }

        // If nothing was copied, remove the tmp dir and return error
        if copied_entries.is_empty() {
            let _ = stdfs::remove_dir_all(&tmp);
            return Err("no files copied during backup".to_string());
        }

        // Write a small manifest so users can inspect backup contents easily
        let manifest_path = tmp.join("backup_manifest.txt");
        let mut manifest_content = format!(
            "backup_time_ms={}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        for e in &copied_entries {
            manifest_content.push_str(&format!("{}\n", e));
        }
        stdfs::write(&manifest_path, manifest_content)
            .map_err(|e| format!("write manifest failed: {}", e))?;

        // Atomically rename tmp -> final dst
        stdfs::rename(&tmp, &dst).map_err(|e| format!("rename tmp to dst failed: {}", e))?;

        Ok(())
    })
    .await
    .map_err(|e| format!("join error: {}", e))?;

    // Regardless of copy result, attempt to re-enable saves if we disabled them.
    if used_rcon {
        if let Err(e) = rcon_save_on(server_root).await {
            warn!("rcon save-on failed: {}", e);
        }
    }

    // If the copy failed, return an error so callers don't assume success.
    if let Err(e) = copy_res {
        warn!("backup copy failed: {}", e);
        return Err(e);
    }

    info!("backup created: {}", backup.display());
    Ok(backup)
}

fn copy_dir_recursive_blocking(src: &Path, dst: &Path) -> Result<(), String> {
    stdfs::create_dir_all(dst).map_err(|e| format!("create_dir_all: {}", e))?;
    for entry in stdfs::read_dir(src).map_err(|e| format!("read_dir: {}", e))? {
        let entry = entry.map_err(|e| format!("read_dir entry: {}", e))?;
        let path = entry.path();
        let dest = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive_blocking(&path, &dest)?;
        } else if path.is_file() {
            stdfs::copy(&path, &dest).map_err(|e| format!("copy: {}", e))?;
        }
    }
    Ok(())
}

pub async fn archive_current_for_force_reinstall(server_root: &Path) -> Result<PathBuf, String> {
    let current = server_root.join("current");
    if !current.exists() {
        return Err("no current dir to archive".to_string());
    }
    let backup_dir = server_root.join(".runner").join("backup");
    let ts = now_millis();
    let backup = backup_dir.join(format!("current-{}", ts));

    let cur = current.clone();
    let back = backup.clone();
    task::spawn_blocking(move || -> Result<(), String> {
        stdfs::create_dir_all(&backup_dir).map_err(|e| format!("create backup dir: {}", e))?;
        stdfs::rename(&cur, &back).map_err(|e| format!("rename failed: {}", e))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("join error: {}", e))??;

    Ok(backup)
}
