use anyhow::{Context, Result};
use protocol::PackBlob;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct Assembler {
    runtime_dir: PathBuf,
}

impl Assembler {
    pub fn new(runtime_dir: PathBuf) -> Self {
        Self { runtime_dir }
    }

    pub async fn assemble(&self, blob: &PackBlob) -> Result<()> {
        // Create runtime directory if it doesn't exist
        fs::create_dir_all(&self.runtime_dir)
            .await
            .context("Failed to create runtime directory")?;

        println!("Writing {} files...", blob.files.len());

        for (rel_path, data) in &blob.files {
            let target_path = self.runtime_dir.join(rel_path);

            // Ensure parent directory exists
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write the file
            // TODO: Handle config safety policy (don't overwrite if modified locally)
            // For now, we'll just write it.
            fs::write(target_path, data)
                .await
                .context(format!("Failed to write file: {}", rel_path))?;
        }

        Ok(())
    }

    pub async fn link_artifacts(
        &self,
        artifacts_dir: &Path,
        manifest: &protocol::Manifest,
    ) -> Result<()> {
        // This will be called to symlink or copy mods from the cache to the runtime/mods directory
        // For now, let's just make sure the mods directory exists
        let mods_dir = self.runtime_dir.join("mods");
        fs::create_dir_all(&mods_dir).await?;

        println!("Linking {} mod files...", manifest.dependencies.len());

        let mut expected = HashSet::new();

        for dep in &manifest.dependencies {
            // TODO: Filter by platform
            let hash = &dep.hash.hex;
            let artifact_source = artifacts_dir.join(hash);

            // We'll use a simple name for the mod jar if possible, or just the hash for now
            // Ideally we'd have a filename in the manifest
            let filename = format!("{}.jar", hash);
            let target_path = mods_dir.join(filename);
            if let Some(name) = target_path.file_name().and_then(|value| value.to_str()) {
                expected.insert(name.to_string());
            }

            if artifact_source.exists() {
                // Copy or symlink. Symlink is better for disk space but might have issues on some FS.
                // Let's copy for maximum compatibility initially.
                fs::copy(artifact_source, target_path).await?;
            }
        }

        let mut entries = fs::read_dir(&mods_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            if file_type.is_file() {
                if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                    if !expected.contains(name) {
                        let _ = fs::remove_file(&path).await;
                    }
                }
            }
        }

        Ok(())
    }
}
