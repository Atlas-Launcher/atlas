use anyhow::{Result, Context};
use crate::rcon::RconClient;
use crate::hub::whitelist::InstanceConfig;
use std::path::PathBuf;

pub async fn exec(command: String, it: bool) -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let _config = InstanceConfig::load(&instance_path).await
        .context("No instance.toml found in current directory")?;
    
    // TODO: Get RCON port and password from server.properties
    let rcon = RconClient::new("127.0.0.1:25575".to_string(), "atlas-rcon-pass".to_string());
    
    if it {
        println!("Interactive shell logic not yet fully implemented. Executing single command instead.");
    }
    
    let response = rcon.execute(&command).await?;
    println!("{}", response);

    Ok(())
}
