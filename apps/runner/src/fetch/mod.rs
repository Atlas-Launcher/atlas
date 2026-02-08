use anyhow::Result;
use crate::cache::Cache;
use reqwest::Client;
use std::sync::Arc;

pub struct Fetcher {
    client: Client,
    cache: Arc<Cache>,
}

impl Fetcher {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self {
            client: Client::new(),
            cache,
        }
    }

    pub async fn fetch_artifact(&self, url: String, expected_hash: String) -> Result<()> {
        if self.cache.exists(&expected_hash).await {
            return Ok(());
        }

        let response = self.client.get(&url).send().await?.error_for_status()?;
        let data = response.bytes().await?;
        
        let actual_hash = self.cache.compute_hash(&data);
        if actual_hash != expected_hash {
            anyhow::bail!("Hash mismatch for {}: expected {}, got {}", url, expected_hash, actual_hash);
        }

        self.cache.store(&data).await?;
        Ok(())
    }

    pub async fn fetch_multiple(&self, artifacts: Vec<(String, String)>) -> Result<()> {
        let mut futures = Vec::new();
        for (url, hash) in artifacts {
            futures.push(self.fetch_artifact(url, hash));
        }

        futures::future::try_join_all(futures).await?;
        Ok(())
    }
}
