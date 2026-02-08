use anyhow::{Result, Context};
use std::path::{PathBuf};
use std::sync::Arc;
use crate::hub::HubClient;
use crate::fetch::Fetcher;
use crate::cache::Cache;
use crate::assemble::Assembler;
// PackBlob removed

pub struct Reconciler {
    hub: Arc<HubClient>,
    fetcher: Arc<Fetcher>,
    cache: Arc<Cache>,
    base_dir: PathBuf,
}

impl Reconciler {
    pub fn new(hub: Arc<HubClient>, fetcher: Arc<Fetcher>, cache: Arc<Cache>, base_dir: PathBuf) -> Self {
        Self { hub, fetcher, cache, base_dir }
    }

    pub async fn reconcile(&self, pack_id: &str, channel: &str) -> Result<()> {
        println!("Reconciling instance for pack: {} (channel: {})", pack_id, channel);

        // 1. Fetch latest blob
        let blob_bytes = self.hub.get_build_blob(pack_id, channel).await
            .context("Failed to fetch build blob")?;
        
        // 2. Decode blob
        let blob = protocol::decode_blob(&blob_bytes)
            .context("Failed to decode build blob")?;

        // 3. Fetch artifacts from manifest
        let mut artifacts = Vec::new();
        for dep in &blob.manifest.dependencies {
            artifacts.push((dep.url.clone(), dep.hash.hex.clone()));
        }
        
        println!("Fetching {} artifacts...", artifacts.len());
        self.fetcher.fetch_multiple(artifacts).await?;

        // 4. Assemble runtime in staging area
        let staging_dir = self.base_dir.join("runtime/staging");
        let assembler = Assembler::new(staging_dir.clone());
        
        println!("Assembling runtime in staging area...");
        assembler.assemble(&blob).await?;
        assembler.link_artifacts(&self.cache.get_path(""), &blob.manifest).await?;

        // 5. Finalize (Stop server, Swap, Start server)
        // This will be implemented when Supervisor is ready
        self.finalize(&staging_dir).await?;

        Ok(())
    }

    async fn finalize(&self, staging_dir: &PathBuf) -> Result<()> {
        let current_dir = self.base_dir.join("runtime/current");
        
        println!("Finalizing deployment (atomic swap)...");
        
        // TODO: Stop server here if running
        
        if current_dir.exists() {
            // Move current to old or delete it
            let old_dir = self.base_dir.join("runtime/old");
            if old_dir.exists() {
                tokio::fs::remove_dir_all(&old_dir).await?;
            }
            tokio::fs::rename(&current_dir, &old_dir).await?;
        }

        tokio::fs::rename(staging_dir, &current_dir).await?;
        
        // TODO: Start server here

        Ok(())
    }
}
