use crate::models::AppSettings;
use crate::paths::{ensure_dir, file_exists, settings_store_path};
use std::fs;

pub fn load_settings() -> Result<AppSettings, String> {
    let path = settings_store_path()?;
    if !file_exists(&path) {
        return Ok(AppSettings::default());
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read settings: {err}"))?;
    let settings = serde_json::from_slice::<AppSettings>(&bytes)
        .map_err(|err| format!("Failed to parse settings: {err}"))?;
    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = settings_store_path()?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_vec_pretty(settings)
        .map_err(|err| format!("Failed to serialize settings: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Failed to write settings: {err}"))?;
    Ok(())
}
