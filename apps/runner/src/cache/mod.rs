use anyhow::{Result, Context};
use std::path::{PathBuf};
use tokio::fs;
use sha2::{Sha256, Digest};

pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn get_path(&self, hash: &str) -> PathBuf {
        self.root.join(hash)
    }

    pub async fn exists(&self, hash: &str) -> bool {
        self.get_path(hash).exists()
    }

    pub async fn store(&self, data: &[u8]) -> Result<String> {
        let hash = self.compute_hash(data);
        let path = self.get_path(&hash);
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(path, data).await?;
        Ok(hash)
    }

    pub fn compute_hash(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub async fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.root).await.context("Failed to create cache directory")
    }
}
