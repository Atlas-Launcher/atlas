use anyhow::Result;
use crate::hub::{HubClient, whitelist::WhitelistSync};
use crate::reconcile::Reconciler;
use crate::supervisor::Supervisor;
use crate::hub::whitelist::InstanceConfig;
use std::sync::Arc;
use std::path::PathBuf;

pub async fn exec(_force_config: bool) -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let config = InstanceConfig::load(&instance_path).await?;
    
    let _hub = Arc::new(HubClient::new(&config.hub_url)?);
    let mut hub_mut = HubClient::new(&config.hub_url)?;
    hub_mut.set_token(config.token.clone());
    let hub = Arc::new(hub_mut);

    let _cache_dir = PathBuf::from("cache");
    let _cache = Arc::new(crate::cache::Cache::new(_cache_dir));
    _cache.init().await?;

    let _fetcher = Arc::new(crate::fetch::Fetcher::new(_cache.clone()));
    
    // Reconcile
    let reconciler = Reconciler::new(hub.clone(), _fetcher.clone(), _cache.clone(), PathBuf::from("."));
    reconciler.reconcile(&config.pack_id, &config.channel).await?;

    // Whitelist sync
    let whitelist = WhitelistSync::new(hub.clone(), PathBuf::from("runtime/current"));
    whitelist.sync(&config.pack_id).await?;

    // Start server
    let jvm_args = vec!["-Xmx6G".to_string(), "-jar".to_string(), "server.jar".to_string(), "nogui".to_string()];
    let supervisor = Supervisor::new(PathBuf::from("runtime/current"), jvm_args);
    
    println!("Starting Minecraft server...");
    let mut child = supervisor.spawn().await?;
    
    // Wait for child to exit
    let status = child.wait().await?;
    println!("Server exited with status: {}", status);

    Ok(())
}
