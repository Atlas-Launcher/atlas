use anyhow::{Result, Context};
use crate::supervisor::Supervisor;
use crate::hub::whitelist::InstanceConfig;
use std::path::PathBuf;

pub async fn exec() -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let config = InstanceConfig::load(&instance_path).await
        .context("No instance.toml found in current directory")?;
    
    let supervisor = Supervisor::new(
        PathBuf::from("runtime/current"),
        "java".to_string(),
        vec![],
        Vec::new(),
    );
    
    println!("Pack: {}", config.pack_id);
    println!("Channel: {}", config.channel);
    
    if supervisor.is_running().await {
        println!("Status: RUNNING");
    } else {
        println!("Status: STOPPED");
    }

    Ok(())
}
