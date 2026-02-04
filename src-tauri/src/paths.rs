use std::path::{Path, PathBuf};

pub fn default_game_dir() -> PathBuf {
  if cfg!(target_os = "windows") {
    if let Ok(appdata) = std::env::var("APPDATA") {
      return PathBuf::from(appdata).join(".minecraft");
    }
  }

  if cfg!(target_os = "macos") {
    if let Some(home) = dirs::home_dir() {
      return home.join("Library").join("Application Support").join("minecraft");
    }
  }

  if let Some(home) = dirs::home_dir() {
    return home.join(".minecraft");
  }

  PathBuf::from(".minecraft")
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
