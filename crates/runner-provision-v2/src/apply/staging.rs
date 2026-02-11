use crate::errors::ProvisionError;
use std::path::{Path, PathBuf};

pub async fn create_staging_dir(server_root: &Path) -> Result<PathBuf, ProvisionError> {
    let base = server_root.join(".runner").join("staging");
    ensure_dir(&base).await?;
    let nonce = format!("{}", crate::now_millis());
    let dir = base.join(nonce);
    ensure_dir(&dir).await?;
    Ok(dir)
}

pub async fn ensure_dir(p: &Path) -> Result<(), ProvisionError> {
    tokio::fs::create_dir_all(p).await?;
    Ok(())
}

pub async fn promote(server_root: &Path, staged_current: &Path) -> Result<(), ProvisionError> {
    let current = server_root.join("current");
    let runner = server_root.join(".runner");
    let backup_dir = runner.join("backup");
    ensure_dir(&backup_dir).await?;

    // If current exists, move to backup (atomic rename)
    if tokio::fs::try_exists(&current).await? {
        let backup = backup_dir.join(format!("current-{}", crate::now_millis()));
        tokio::fs::rename(&current, &backup).await?;
    }

    // Promote staged_current -> current
    tokio::fs::rename(staged_current, &current).await?;

    Ok(())
}
