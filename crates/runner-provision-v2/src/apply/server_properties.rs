use std::path::Path;

use rand::RngCore;

use crate::errors::ProvisionError;

pub async fn ensure_whitelist_enforced(runtime_dir: &Path) -> Result<(), ProvisionError> {
    let server_props = runtime_dir.join("server.properties");
    let mut current = tokio::fs::read_to_string(&server_props).await.unwrap_or_default();
    if current.trim().is_empty() {
        current = String::new();
    }

    let mut updated = set_property(&current, "white-list", "true");
    updated = set_property(&updated, "enforce-whitelist", "true");

    tokio::fs::write(&server_props, updated).await?;
    Ok(())
}

pub async fn ensure_rcon_configured(runtime_dir: &Path) -> Result<(), ProvisionError> {
    let server_props = runtime_dir.join("server.properties");
    let mut current = tokio::fs::read_to_string(&server_props).await.unwrap_or_default();
    if current.trim().is_empty() {
        current = String::new();
    }

    let mut updated = set_property(&current, "enable-rcon", "true");
    let rcon_port = get_property(&updated, "rcon.port").unwrap_or_else(|| "25575".to_string());
    updated = set_property(&updated, "rcon.port", rcon_port.trim());

    let password = match get_property(&updated, "rcon.password") {
        Some(value) if !value.trim().is_empty() => value,
        _ => generate_rcon_password(),
    };
    updated = set_property(&updated, "rcon.password", password.trim());

    tokio::fs::write(&server_props, updated).await?;
    Ok(())
}

fn get_property(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix(&prefix) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn set_property(contents: &str, key: &str, value: &str) -> String {
    let mut lines = Vec::new();
    let mut replaced = false;
    let prefix = format!("{}=", key);
    for line in contents.lines() {
        if line.trim_start().starts_with(&prefix) {
            lines.push(format!("{}={}", key, value));
            replaced = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced {
        lines.push(format!("{}={}", key, value));
    }
    format!("{}\n", lines.join("\n"))
}

fn generate_rcon_password() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
