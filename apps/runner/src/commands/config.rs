use crate::hub::whitelist::InstanceConfig;
use crate::runner_config;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn exec(
    memory: Option<String>,
    port: Option<u16>,
    java_major: Option<u32>,
) -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let mut config = InstanceConfig::load(&instance_path)
        .await
        .context("No instance.toml found. Run `atlas-runner auth` first.")?;

    if let Some(value) = memory {
        config.memory = Some(value);
    } else if config.memory.is_none() {
        config.memory = Some(runner_config::default_memory()?);
    }

    if let Some(value) = port {
        config.port = Some(value);
    }

    if let Some(value) = java_major {
        config.java_major = Some(value);
    }

    config.save(&instance_path).await?;
    println!("Instance configuration updated.");
    Ok(())
}
