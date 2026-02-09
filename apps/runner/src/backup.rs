use anyhow::{Context, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn archive_worlds(
    current_dir: &Path,
    archive_dir: &Path,
    prefix: &str,
    keep: usize,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(archive_dir).await?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let archive_path = archive_dir.join(format!("{prefix}-{timestamp}.tar.gz"));

    let current_dir = current_dir.to_path_buf();
    let archive_path_clone = archive_path.clone();
    let prefix = prefix.to_string();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = std::fs::File::create(&archive_path_clone)?;
        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(encoder);

        for entry in std::fs::read_dir(&current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("world") {
                continue;
            }
            tar.append_dir_all(name.as_ref(), &path)?;
        }

        tar.finish()?;
        Ok(())
    })
    .await
    .context("Failed to archive worlds")??;

    if keep > 0 {
        prune_archives(archive_dir, &prefix, keep).await?;
    }

    Ok(archive_path)
}

async fn prune_archives(archive_dir: &Path, prefix: &str, keep: usize) -> Result<()> {
    let mut entries = fs::read_dir(archive_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.starts_with(prefix) && name.ends_with(".tar.gz")
        })
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.file_name());
    if entries.len() <= keep {
        return Ok(());
    }

    let remove_count = entries.len() - keep;
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path());
    }

    Ok(())
}
