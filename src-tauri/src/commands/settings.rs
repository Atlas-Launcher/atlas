use crate::models::AppSettings;
use crate::paths;
use crate::settings;
use crate::state::AppState;

#[tauri::command]
pub fn get_default_game_dir() -> String {
    paths::default_game_dir().to_string_lossy().to_string()
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
