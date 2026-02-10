use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployKeyConfig {
    pub hub_url: String,
    pub pack_id: String,
    #[serde(default = "default_channel")]
    pub channel: String,
    pub deploy_key: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

pub fn save_deploy_key(config: &DeployKeyConfig) -> Result<(), String> {
    let path = deploy_key_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create runnerd config dir: {err}"))?;
    }

    let payload = serde_json::to_string_pretty(config)
        .map_err(|err| format!("Failed to serialize deploy key config: {err}"))?;
    fs::write(&path, payload)
        .map_err(|err| format!("Failed to write deploy key config: {err}"))?;

    Ok(())
}

pub fn load_deploy_key() -> Result<Option<DeployKeyConfig>, String> {
    let path = deploy_key_path()?;
    let content = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };

    let config = serde_json::from_str::<DeployKeyConfig>(&content)
        .map_err(|err| format!("Failed to parse deploy key config: {err}"))?;
    Ok(Some(config))
}

fn deploy_key_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("deploy.json"))
}

fn config_dir() -> Result<PathBuf, String> {
    if let Some(base) = dirs::data_dir() {
        return Ok(base.join("atlas").join("runnerd"));
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".atlas").join("runnerd"));
    }
    Err("Unable to resolve a writable data directory".to_string())
}

fn default_channel() -> String {
    "production".to_string()
}
