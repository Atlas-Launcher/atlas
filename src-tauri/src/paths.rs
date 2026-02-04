use std::path::{Path, PathBuf};

pub fn default_game_dir() -> PathBuf {
  if let Some(data) = dirs::data_dir() {
    return data.join("mc-launcher").join("game");
  }

  if let Some(home) = dirs::home_dir() {
    return home.join(".mc-launcher").join("game");
  }

  PathBuf::from("mc-launcher-game")
}

pub fn ensure_dir(path: &Path) -> Result<(), String> {
  std::fs::create_dir_all(path).map_err(|err| format!("Failed to create dir {}: {err}", path.display()))
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
    return Ok(base.join("mc-launcher"));
  }
  if let Some(home) = dirs::home_dir() {
    return Ok(home.join(".mc-launcher"));
  }
  Err("Unable to resolve a writable data directory".to_string())
}

pub fn auth_store_path() -> Result<PathBuf, String> {
  Ok(auth_store_dir()?.join("auth.json"))
}
