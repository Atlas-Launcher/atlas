use anyhow::{Context, Result};
use minecraft_client_rs::Client;
use std::path::Path;
use tokio::fs;
use tokio::task::spawn_blocking;

pub struct RconSettings {
    pub address: String,
    pub password: String,
}

pub struct RconClient {
    address: String,
    password: String,
}

impl RconClient {
    pub fn new(address: String, password: String) -> Self {
        Self { address, password }
    }

    pub async fn execute(&self, command: &str) -> Result<String> {
        let address = self.address.clone();
        let password = self.password.clone();
        let command = command.to_string();

        spawn_blocking(move || {
            let mut client = Client::new(address)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            client
                .authenticate(password)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            let response = client
                .send_command(command)
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            client
                .close()
                .map_err(|err| anyhow::anyhow!(err.to_string()))?;
            Ok::<_, anyhow::Error>(response.body)
        })
        .await
        .with_context(|| "RCON task failed")?
    }
}

pub async fn load_rcon_settings(runtime_dir: &Path) -> Result<Option<RconSettings>> {
    let properties_path = runtime_dir.join("server.properties");
    let content = match fs::read_to_string(&properties_path).await {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let mut enabled = false;
    let mut port: Option<u16> = None;
    let mut password: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        match key.trim() {
            "enable-rcon" => enabled = value.trim().eq_ignore_ascii_case("true"),
            "rcon.port" => port = value.trim().parse::<u16>().ok(),
            "rcon.password" => {
                let val = value.trim();
                if !val.is_empty() {
                    password = Some(val.to_string());
                }
            }
            _ => {}
        }
    }

    if !enabled {
        return Ok(None);
    }

    let password = password.ok_or_else(|| anyhow::anyhow!("Missing rcon.password"))?;
    let address = format!("127.0.0.1:{}", port.unwrap_or(25575));
    Ok(Some(RconSettings { address, password }))
}
