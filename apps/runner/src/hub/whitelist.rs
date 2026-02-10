use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use crate::hub::{HubClient, WhitelistEntry};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceConfig {
    pub pack_id: String,
    pub channel: String,
    pub hub_url: String,
    pub token: Option<String>,
    pub service_token: Option<String>,
    pub memory: Option<String>,
    pub port: Option<u16>,
    pub minecraft_version: Option<String>,
    pub java_major: Option<u32>,
    pub modloader: Option<String>,
    pub modloader_version: Option<String>,
}


impl InstanceConfig {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).await.context("Failed to read instance.toml")?;
        let config: Self = toml::from_str(&content).context("Failed to parse instance.toml")?;
        Ok(config)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize instance.toml")?;
        let _: () = fs::write(path, content).await.context("Failed to write instance.toml")?;
        Ok(())
    }
}

pub struct WhitelistSync {
    hub: Arc<HubClient>,
    runtime_dir: PathBuf,
    current_etag: std::sync::Mutex<Option<String>>,
}

impl WhitelistSync {
    pub fn new(hub: Arc<HubClient>, runtime_dir: PathBuf) -> Self {
        Self { 
            hub, 
            runtime_dir,
            current_etag: std::sync::Mutex::new(None),
        }
    }

    pub async fn sync(&self, pack_id: &str) -> Result<bool> {
        let current_etag = self.current_etag.lock().unwrap().clone();
        println!("Syncing whitelist from Hub... (current etag: {:?})", current_etag);
        
        let (players, new_etag) = self.hub.get_whitelist_with_version(pack_id, current_etag.as_deref()).await?;
        
        if players.is_empty() && current_etag.is_some() {
            // ETag matched, no update needed
            println!("Whitelist is up to date (etag: {})", current_etag.as_ref().unwrap());
            return Ok(false);
        }
        
        let whitelist_data = players
            .into_iter()
            .map(|player| {
                serde_json::json!({
                    "name": player.name,
                    "uuid": player.uuid,
                })
            })
            .collect::<Vec<_>>();

        let path = self.runtime_dir.join("whitelist.json");
        let content = serde_json::to_string_pretty(&whitelist_data)?;
        let previous = fs::read_to_string(&path).await.ok();
        if previous.as_deref() == Some(content.as_str()) {
            println!("Whitelist already up to date.");
            return Ok(false);
        }

        fs::write(path, content).await.context("Failed to write whitelist.json")?;
        
        // Update stored etag
        *self.current_etag.lock().unwrap() = Some(new_etag.clone());
        
        println!("Whitelist synchronized (etag: {})", new_etag);
        Ok(true)
    }
}
