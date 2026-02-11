use crate::RUNNER_BASE_DIR;
use crate::hub::whitelist::InstanceConfig;
use crate::rcon::{RconClient, load_rcon_settings};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn exec(command: String, it: bool) -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let _config = InstanceConfig::load(&instance_path)
        .await
        .context("No instance.toml found in current directory")?;

    let runtime_dir = PathBuf::from(RUNNER_BASE_DIR).join("runtime/current");
    let settings = load_rcon_settings(&runtime_dir)
        .await
        .context("RCON not configured in server.properties")?;
    let settings = settings.context("RCON not enabled")?;
    let rcon = RconClient::new(settings.address, settings.password);

    if it {
        println!(
            "Interactive shell logic not yet fully implemented. Executing single command instead."
        );
    }

    let response = rcon.execute(&command).await?;
    println!("{}", response);

    Ok(())
}
