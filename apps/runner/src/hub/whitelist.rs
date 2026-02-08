use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use crate::hub::HubClient;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstanceConfig {
    pub pack_id: String,
    pub channel: String,
    pub hub_url: String,
    pub token: String,
    pub memory: Option<String>,
    pub port: Option<u16>,
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
}

impl WhitelistSync {
    pub fn new(hub: Arc<HubClient>, runtime_dir: PathBuf) -> Self {
        Self { hub, runtime_dir }
    }

    pub async fn sync(&self, pack_id: &str) -> Result<()> {
        println!("Syncing whitelist from Hub...");
        let players = self.hub.get_whitelist(pack_id).await?;
        
        let mut whitelist_data = Vec::new();
        for player in players {
            // Simplified whitelist.json entry (only name for now, usually needs UUID)
            // Minecraft actually expects a JSON array of objects with "uuid" and "name"
            // For now, let's just write names or handle common format
            whitelist_data.push(serde_json::json!({
                "name": player,
                "uuid": "" // We'd need to resolve UUIDs or get them from Hub
            }));
        }

        let path = self.runtime_dir.join("whitelist.json");
        let content = serde_json::to_string_pretty(&whitelist_data)?;
        fs::write(path, content).await.context("Failed to write whitelist.json")?;
        
        println!("Whitelist synchronized.");
        Ok(())
    }
}
