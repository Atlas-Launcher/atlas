use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use atlas_auth::device_code::{
    DEFAULT_ATLAS_DEVICE_CLIENT_ID, DEFAULT_ATLAS_HUB_URL, normalize_hub_url,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliAuthSession {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: u64,
    pub hub_url: String,
    pub client_id: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    pub created_at: u64,
}

pub fn resolve_hub_url(hub_url_override: Option<String>) -> String {
    normalize_optional(hub_url_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_HUB_URL").ok()))
        .map(|value| normalize_hub_url(&value))
        .unwrap_or_else(|| DEFAULT_ATLAS_HUB_URL.to_string())
}

pub fn resolve_device_client_id(client_id_override: Option<String>) -> String {
    normalize_optional(client_id_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_DEVICE_CLIENT_ID").ok()))
        .unwrap_or_else(|| DEFAULT_ATLAS_DEVICE_CLIENT_ID.to_string())
}

pub fn load_cli_auth_session() -> Result<Option<CliAuthSession>> {
    let path = auth_store_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let session = serde_json::from_slice::<CliAuthSession>(&bytes)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(session))
}

pub fn save_cli_auth_session(session: &CliAuthSession) -> Result<()> {
    let path = auth_store_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let payload =
        serde_json::to_vec_pretty(session).context("Failed to serialize auth session file")?;
    fs::write(&path, payload).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn remove_cli_auth_session() -> Result<()> {
    let path = auth_store_path()?;
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("Failed to remove {}", path.display()))?;
    }
    Ok(())
}

pub fn require_access_token_for_hub(hub_url: &str) -> Result<String> {
    let requested_hub = normalize_hub_url(hub_url);
    let session = load_cli_auth_session()?
        .context("No CLI auth session found. Run `atlas auth signin` first.")?;

    if normalize_hub_url(&session.hub_url) != requested_hub {
        bail!(
            "CLI auth session is for {} but current hub is {}. Run `atlas auth signin --hub-url {}`.",
            session.hub_url,
            requested_hub,
            requested_hub
        );
    }

    let now = unix_timestamp();
    if session.expires_at > 0 && now + 30 >= session.expires_at {
        bail!("CLI auth session expired. Run `atlas auth signin` again.");
    }

    Ok(session.access_token)
}

pub fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn auth_store_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Could not resolve home directory")?;
    Ok(home_dir.join(".atlas").join("cli-auth.json"))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
