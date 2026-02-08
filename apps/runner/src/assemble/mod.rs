use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use tokio::fs;
use protocol::PackBlob;

pub struct Assembler {
    runtime_dir: PathBuf,
}

impl Assembler {
    pub fn new(runtime_dir: PathBuf) -> Self {
        Self { runtime_dir }
    }

    pub async fn assemble(&self, blob: &PackBlob) -> Result<()> {
        // Create runtime directory if it doesn't exist
        fs::create_dir_all(&self.runtime_dir).await.context("Failed to create runtime directory")?;

        for (rel_path, data) in &blob.files {
            let target_path = self.runtime_dir.join(rel_path);
            
            // Ensure parent directory exists
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write the file
            // TODO: Handle config safety policy (don't overwrite if modified locally)
            // For now, we'll just write it.
            fs::write(target_path, data).await.context(format!("Failed to write file: {}", rel_path))?;
        }

        Ok(())
    }

    pub async fn link_artifacts(&self, artifacts_dir: &Path, manifest: &protocol::Manifest) -> Result<()> {
        // This will be called to symlink or copy mods from the cache to the runtime/mods directory
        // For now, let's just make sure the mods directory exists
        let mods_dir = self.runtime_dir.join("mods");
        fs::create_dir_all(&mods_dir).await?;

        for dep in &manifest.dependencies {
            // TODO: Filter by platform
            let hash = &dep.hash.hex;
            let artifact_source = artifacts_dir.join(hash);
            
            // We'll use a simple name for the mod jar if possible, or just the hash for now
            // Ideally we'd have a filename in the manifest
            let filename = format!("{}.jar", hash); 
            let target_path = mods_dir.join(filename);

            if artifact_source.exists() {
                // Copy or symlink. Symlink is better for disk space but might have issues on some FS.
                // Let's copy for maximum compatibility initially.
                fs::copy(artifact_source, target_path).await?;
            }
        }

        Ok(())
    }
}
