mod auth;
mod launcher;
mod models;
mod paths;
mod state;

use crate::models::{DeviceCodeResponse, LaunchOptions, Profile};
use crate::state::AppState;

const DEFAULT_MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

fn resolve_client_id(raw: String) -> String {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    DEFAULT_MS_CLIENT_ID.to_string()
  } else {
    trimmed.to_string()
  }
}

#[tauri::command]
fn get_default_game_dir() -> String {
  paths::default_game_dir().to_string_lossy().to_string()
}

#[tauri::command]
async fn start_device_code(client_id: String) -> Result<DeviceCodeResponse, String> {
  let resolved = resolve_client_id(client_id);
  auth::start_device_code(&resolved).await
}

#[tauri::command]
async fn complete_device_code(
  state: tauri::State<'_, AppState>,
  client_id: String,
  device_code: String
) -> Result<Profile, String> {
  let resolved = resolve_client_id(client_id);
  let session = auth::complete_device_code(&resolved, &device_code).await?;
  let profile = session.profile.clone();
  auth::save_session(&session)?;
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
  let mut session = state
    .auth
    .lock()
    .map_err(|_| "Auth state lock poisoned".to_string())?
    .clone();

  if session.is_none() {
    session = auth::load_session()?;
  }

  let mut session =
    session.ok_or_else(|| "Not signed in. Start device login first.".to_string())?;

  if session.client_id.trim().is_empty() {
    session.client_id = DEFAULT_MS_CLIENT_ID.to_string();
  }

  let session = auth::ensure_fresh_session(session).await?;
  auth::save_session(&session)?;

  {
    let mut guard = state
      .auth
      .lock()
      .map_err(|_| "Auth state lock poisoned".to_string())?;
    *guard = Some(session.clone());
  }

  launcher::launch_minecraft(&window, &options, &session).await
}

#[tauri::command]
async fn download_minecraft_files(
  window: tauri::Window,
  options: LaunchOptions
) -> Result<(), String> {
  launcher::download_minecraft_files(&window, &options).await
}

#[tauri::command]
async fn restore_session(state: tauri::State<'_, AppState>) -> Result<Option<Profile>, String> {
  let session = auth::load_session()?;
  let Some(mut session) = session else {
    return Ok(None);
  };

  if session.client_id.trim().is_empty() {
    session.client_id = DEFAULT_MS_CLIENT_ID.to_string();
  }

  let session = auth::ensure_fresh_session(session).await?;
  auth::save_session(&session)?;

  let profile = session.profile.clone();
  let mut guard = state
    .auth
    .lock()
    .map_err(|_| "Auth state lock poisoned".to_string())?;
  *guard = Some(session);
  Ok(Some(profile))
}

#[tauri::command]
fn sign_out(state: tauri::State<'_, AppState>) -> Result<(), String> {
  auth::clear_session()?;
  let mut guard = state
    .auth
    .lock()
    .map_err(|_| "Auth state lock poisoned".to_string())?;
  *guard = None;
  Ok(())
}

fn main() {
  tauri::Builder::default()
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![
      get_default_game_dir,
      start_device_code,
      complete_device_code,
      launch_minecraft,
      download_minecraft_files,
      restore_session,
      sign_out
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
