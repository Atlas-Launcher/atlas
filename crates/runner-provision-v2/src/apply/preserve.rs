use std::path::{Path, PathBuf};
use crate::errors::ProvisionError;
use futures_util::future::BoxFuture;

pub async fn preserve_from_existing(server_root: &Path, staging_current: &Path) -> Result<(), ProvisionError> {
    let current = server_root.join("current");
    if !tokio::fs::try_exists(&current).await? {
        return Ok(());
    }

    // Preserve worlds
    for name in ["world", "world_nether", "world_the_end"] {
        copy_dir_if_exists(current.join(name), staging_current.join(name)).await?;
    }

    // Preserve identity files
    for name in ["whitelist.json", "ops.json", "banned-ips.json", "banned-players.json", "usercache.json"] {
        copy_file_if_exists(current.join(name), staging_current.join(name)).await?;
    }

    Ok(())
}

async fn copy_file_if_exists(src: PathBuf, dst: PathBuf) -> Result<(), ProvisionError> {
    if tokio::fs::try_exists(&src).await? {
        if let Some(p) = dst.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        tokio::fs::copy(src, dst).await?;
    }
    Ok(())
}

fn copy_dir_if_exists(src: PathBuf, dst: PathBuf) -> BoxFuture<'static, Result<(), ProvisionError>> {
    Box::pin(async move {
        if !tokio::fs::try_exists(&src).await? {
            return Ok(());
        }
        // Minimal recursive copy (you can replace with a crate later)
        tokio::fs::create_dir_all(&dst).await?;
        let mut rd = tokio::fs::read_dir(&src).await?;
        while let Some(ent) = rd.next_entry().await? {
            let ft = ent.file_type().await?;
            let s = ent.path();
            let d = dst.join(ent.file_name());
            if ft.is_dir() {
                copy_dir_if_exists(s, d).await?;
            } else if ft.is_file() {
                tokio::fs::copy(&s, &d).await?;
            }
        }
        Ok(())
    })
}
