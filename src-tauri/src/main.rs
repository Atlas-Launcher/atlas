mod auth;
mod launcher;
mod models;
mod paths;
mod state;

use crate::models::{DeviceCodeResponse, LaunchOptions, Profile};
use crate::state::AppState;

#[tauri::command]
fn get_default_game_dir() -> String {
  paths::default_game_dir().to_string_lossy().to_string()
}

#[tauri::command]
async fn start_device_code(client_id: String) -> Result<DeviceCodeResponse, String> {
  auth::start_device_code(&client_id).await
}

#[tauri::command]
async fn complete_device_code(
  state: tauri::State<'_, AppState>,
  client_id: String,
  device_code: String
) -> Result<Profile, String> {
  let session = auth::complete_device_code(&client_id, &device_code).await?;
  let profile = session.profile.clone();
  let mut guard = state
    .auth
    .lock()
    .map_err(|_| "Auth state lock poisoned".to_string())?;
  *guard = Some(session);
  Ok(profile)
}

#[tauri::command]
async fn launch_minecraft(
  window: tauri::Window,
  state: tauri::State<'_, AppState>,
  options: LaunchOptions
) -> Result<(), String> {
  let session = state
    .auth
    .lock()
    .map_err(|_| "Auth state lock poisoned".to_string())?
    .clone()
    .ok_or_else(|| "Not signed in".to_string())?;

  launcher::launch_minecraft(&window, &options, &session).await
}

fn main() {
  tauri::Builder::default()
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![
      get_default_game_dir,
      start_device_code,
      complete_device_code,
      launch_minecraft
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
