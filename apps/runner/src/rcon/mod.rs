use anyhow::{Result, Context};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs;
use std::path::Path;

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
        let mut stream = TcpStream::connect(&self.address).await
            .context("Failed to connect to RCON")?;

        // 1. Authenticate
        self.authenticate(&mut stream).await?;

        // 2. Send command
        self.send_packet(&mut stream, 2, command).await?;

        // 3. Receive response
        let response = self.receive_packet(&mut stream).await?;
        Ok(response.1)
    }

    async fn authenticate(&self, stream: &mut TcpStream) -> Result<()> {
        self.send_packet(stream, 3, &self.password).await?;
        let (id, _) = self.receive_packet(stream).await?;
        
        if id == -1 {
            anyhow::bail!("RCON Authentication failed");
        }
        Ok(())
    }

    async fn send_packet(&self, stream: &mut TcpStream, type_: i32, payload: &str) -> Result<()> {
        let id: i32 = 42; // arbitrary
        let payload_bytes = payload.as_bytes();
        let size = (10 + payload_bytes.len()) as i32;

        stream.write_i32_le(size).await?;
        stream.write_i32_le(id).await?;
        stream.write_i32_le(type_).await?;
        stream.write_all(payload_bytes).await?;
        stream.write_all(&[0, 0]).await?; // two null bytes
        Ok(())
    }

    async fn receive_packet(&self, stream: &mut TcpStream) -> Result<(i32, String)> {
        let size = stream.read_i32_le().await?;
        let id = stream.read_i32_le().await?;
        let _type_ = stream.read_i32_le().await?;
        
        let mut payload = vec![0u8; (size - 10) as usize];
        stream.read_exact(&mut payload).await?;
        
        // Skip null terminators
        let mut trailer = [0u8; 2];
        stream.read_exact(&mut trailer).await?;

        let body = String::from_utf8_lossy(&payload).to_string();
        Ok((id, body))
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
        let Some((key, value)) = trimmed.split_once('=') else { continue };
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
