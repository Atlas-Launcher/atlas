use crate::auth;
use crate::config;
use crate::launcher;
use crate::models::LaunchOptions;
use crate::state::AppState;

#[tauri::command]
pub async fn launch_minecraft(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    options: LaunchOptions,
) -> Result<(), String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())?
        .clone();
    let client_id = config::resolve_client_id(&settings);
    let mut session = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?
        .clone();

    if session.is_none() {
        session = auth::load_session().map_err(|err| err.to_string())?;
    }

    let mut session = session.ok_or_else(|| "Not signed in. Sign in first.".to_string())?;

    if session.client_id.trim().is_empty() {
        session.client_id = client_id;
    }

    let session = auth::ensure_fresh_session(session)
        .await
        .map_err(|err| err.to_string())?;
    auth::save_session(&session).map_err(|err| err.to_string())?;

    {
        let mut guard = state
            .auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        *guard = Some(session.clone());
    }

    launcher::launch_minecraft(&window, &options, &session)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn download_minecraft_files(
    window: tauri::Window,
    options: LaunchOptions,
) -> Result<(), String> {
    launcher::download_minecraft_files(&window, &options)
        .await
        .map_err(|err| err.to_string())
}
