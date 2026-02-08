use anyhow::{Result, Context};
use crate::supervisor::Supervisor;
use crate::hub::whitelist::InstanceConfig;
use std::path::PathBuf;

pub async fn exec() -> Result<()> {
    let instance_path = PathBuf::from("instance.toml");
    let _config = InstanceConfig::load(&instance_path).await
        .context("No instance.toml found in current directory")?;
    
    // For now we don't need the whole config for 'down'
    let supervisor = Supervisor::new(PathBuf::from("runtime/current"), vec![]);
    
    if supervisor.is_running().await {
        println!("Stopping Minecraft server...");
        // TODO: Try RCON "stop" first
        
        // Use a dummy child object to kill the process if we have the PID
        let pid_file = PathBuf::from("runtime/current/server.pid");
        if let Ok(pid_str) = tokio::fs::read_to_string(&pid_file).await {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // On unix we can use libc or just kill command
                std::process::Command::new("kill")
                    .arg(pid.to_string())
                    .status()?;
                println!("Server stopped.");
                let _ = tokio::fs::remove_file(pid_file).await;
            }
        }
    } else {
        println!("Server is not running.");
    }

    Ok(())
}
