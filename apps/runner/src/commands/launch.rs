use anyhow::{Result, bail};
use crate::hub::{HubClient};
use crate::hub::whitelist::InstanceConfig;
use crate::commands::up;
use std::time::Duration;
use std::path::PathBuf;

pub async fn exec(
    hub_url: &str,
    pack_id: String,
    channel: String,
    memory: Option<String>,
    port: Option<u16>,
    _accept_eula: bool,
) -> Result<()> {
    let mut hub = HubClient::new(hub_url)?;

    // Start login flow
    let device_code = hub.login().await?;
    println!("To authorize, please visit: {}", device_code.verification_uri_complete.as_ref().unwrap_or(&device_code.verification_uri));
    println!("User code: {}", device_code.user_code);

    // Poll for token
    let token;
    let mut interval = Duration::from_secs(device_code.interval);
    if interval.as_secs() == 0 {
        interval = Duration::from_secs(5);
    }

    loop {
        tokio::time::sleep(interval).await;
        match hub.poll_token(&device_code.device_code).await {
            Ok(Some(t)) => {
                token = t;
                break;
            }
            Ok(None) => continue,
            Err(e) => bail!("Authentication failed: {}", e),
        }
    }

    println!("Successfully authenticated!");

    // Check permissions
    if !hub.check_creator_permission(&pack_id).await? {
        bail!("You do not have permission to manage this pack (creator role required)");
    }

    // Save instance state
    let config = InstanceConfig {
        pack_id: pack_id.clone(),
        channel: channel.clone(),
        hub_url: hub_url.to_string(),
        token,
        memory,
        port,
    };
    
    config.save(&PathBuf::from("instance.toml")).await?;
    println!("Instance configuration saved to instance.toml");

    // Initial up
    up::exec(false).await?;
    
    Ok(())
}
