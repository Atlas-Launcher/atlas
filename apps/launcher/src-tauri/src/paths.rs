use std::path::{Path, PathBuf};

pub fn default_game_dir() -> PathBuf {
    if let Some(data) = dirs::data_dir() {
        return data.join("atlas").join("game");
    }

    if let Some(home) = dirs::home_dir() {
        return home.join(".atlas").join("game");
    }

    PathBuf::from("atlas-game")
}

pub fn ensure_dir(path: &Path) -> Result<(), String> {
    std::fs::create_dir_all(path)
        .map_err(|err| format!("Failed to create dir {}: {err}", path.display()))
}

pub fn file_exists(path: &Path) -> bool {
    std::fs::metadata(path).is_ok()
}

pub fn normalize_path(path: &str) -> PathBuf {
    if path.trim().is_empty() {
        return default_game_dir();
    }
    PathBuf::from(path)
}

pub fn auth_store_dir() -> Result<PathBuf, String> {
    if let Some(base) = dirs::data_dir() {
        return Ok(base.join("atlas"));
    }
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".atlas"));
    }
    Err("Unable to resolve a writable data directory".to_string())
}

pub fn auth_store_path() -> Result<PathBuf, String> {
    Ok(auth_store_dir()?.join("auth.json"))
}

pub fn atlas_auth_store_path() -> Result<PathBuf, String> {
    Ok(auth_store_dir()?.join("atlas_auth.json"))
}

pub fn settings_store_path() -> Result<PathBuf, String> {
    Ok(auth_store_dir()?.join("settings.json"))
}
