use crate::models::AppSettings;
use crate::paths;
use crate::settings;
use crate::state::AppState;
use sysinfo::System;

#[tauri::command]
pub fn get_default_game_dir() -> String {
    paths::default_game_dir().to_string_lossy().to_string()
}

#[tauri::command]
pub fn get_system_memory_mb() -> Result<u64, String> {
    let mut system = System::new();
    system.refresh_memory();
    let total_mb = system.total_memory() / 1024 / 1024;
    if total_mb == 0 {
        return Err("Failed to detect system memory".to_string());
    }
    Ok(total_mb)
}

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Result<AppSettings, String> {
    let guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    settings::save_settings(&settings)?;
    let mut guard = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?;
    *guard = settings;
    Ok(())
}
