use crate::auth;
use crate::auth::AuthError;
use crate::diagnostics;
use crate::models::{
    AtlasSession, AuthSession, FixAction, FixResult, LaunchReadinessReport, RepairResult,
    SupportBundleResult, TroubleshooterReport,
};
use crate::settings;
use crate::state::AppState;

// Attempt to load and ensure a fresh Microsoft auth session. This will try
// the in-memory state first, then the on-disk session. If a refresh attempt
// fails we clear the stored session and return None so downstream logic does
// not treat an expired/invalid session as 'signed in'.
async fn load_auth_session_fresh(
    state: tauri::State<'_, AppState>,
) -> Result<Option<AuthSession>, AuthError> {
    // Try in-memory first: clone out the Option<AuthSession> while holding the lock,
    // then drop the MutexGuard before performing any await so the future is Send.
    let maybe_session = {
        let guard = state
            .auth
            .lock()
            .map_err(|_| "Auth state lock poisoned".to_string())?;
        guard.clone()
    };
    if let Some(session) = maybe_session {
        match auth::ensure_fresh_session(session.clone()).await {
            Ok(fresh) => {
                // Persist and update state (lock again after await)
                let _ = auth::save_session(&fresh).map_err(|e| e.to_string())?;
                let mut guard2 = state
                    .auth
                    .lock()
                    .map_err(|_| "Auth state lock poisoned".to_string())?;
                *guard2 = Some(fresh.clone());
                return Ok(Some(fresh));
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.to_lowercase().contains("entitlement") || msg.to_lowercase().contains("minecraft entitlement") {
                    return Err("Microsoft account does not own Minecraft Java Edition.".to_string());
                }
                let _ = auth::clear_session();
                let mut guard2 = state
                    .auth
                    .lock()
                    .map_err(|_| "Auth state lock poisoned".to_string())?;
                *guard2 = None;
                return Ok(None);
            }
        }
    }

    // Fallback to loading from disk
    match auth::load_session().map_err(|e| e.to_string())? {
        Some(session) => match auth::ensure_fresh_session(session).await {
            Ok(fresh) => {
                let _ = auth::save_session(&fresh).map_err(|e| e.to_string())?;
                let mut guard = state
                    .auth
                    .lock()
                    .map_err(|_| "Auth state lock poisoned".to_string())?;
                *guard = Some(fresh.clone());
                Ok(Some(fresh))
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.to_lowercase().contains("entitlement") || msg.to_lowercase().contains("minecraft entitlement") {
                    return Err("Microsoft account does not own Minecraft Java Edition.".to_string());
                }
                let _ = auth::clear_session();
                Ok(None)
            }
        },
        None => Ok(None),
    }
}

// Similar logic for Atlas session: ensure the atlas session is fresh before
// returning it to readiness logic.
async fn load_atlas_session_fresh(
    state: tauri::State<'_, AppState>,
) -> Result<Option<AtlasSession>, String> {
    // Try in-memory first: clone out the Option<AtlasSession> while holding the lock,
    // then drop the MutexGuard before performing any await so the future is Send.
    let maybe_atlas_session = {
        let guard = state
            .atlas_auth
            .lock()
            .map_err(|_| "Atlas auth state lock poisoned".to_string())?;
        guard.clone()
    };
    if let Some(session) = maybe_atlas_session {
        match auth::ensure_fresh_atlas_session(session.clone()).await {
            Ok(fresh) => {
                let _ = auth::save_atlas_session(&fresh).map_err(|e| e.to_string())?;
                let mut guard2 = state
                    .atlas_auth
                    .lock()
                    .map_err(|_| "Atlas auth state lock poisoned".to_string())?;
                *guard2 = Some(fresh.clone());
                return Ok(Some(fresh));
            }
            Err(_) => {
                let _ = auth::clear_atlas_session();
                let mut guard2 = state
                    .atlas_auth
                    .lock()
                    .map_err(|_| "Atlas auth state lock poisoned".to_string())?;
                *guard2 = None;
                return Ok(None);
            }
        }
    }

    match auth::load_atlas_session().map_err(|e| e.to_string())? {
        Some(session) => match auth::ensure_fresh_atlas_session(session).await {
            Ok(fresh) => {
                let _ = auth::save_atlas_session(&fresh).map_err(|e| e.to_string())?;
                let mut guard = state
                    .atlas_auth
                    .lock()
                    .map_err(|_| "Atlas auth state lock poisoned".to_string())?;
                *guard = Some(fresh.clone());
                Ok(Some(fresh))
            }
            Err(_) => {
                let _ = auth::clear_atlas_session();
                Ok(None)
            }
        },
        None => Ok(None),
    }
}

fn load_settings(state: &tauri::State<'_, AppState>) -> Result<crate::models::AppSettings, String> {
    state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())
        .map(|guard| guard.clone())
}

#[tauri::command]
pub async fn get_launch_readiness(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
) -> Result<LaunchReadinessReport, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session_fresh(state.clone()).await?;
    let auth_session = load_auth_session_fresh(state.clone()).await?;
    Ok(diagnostics::build_launch_readiness(
        diagnostics::ReadinessContext {
            settings,
            atlas_session,
            auth_session,
            game_dir,
        },
    ))
}

#[tauri::command]
pub async fn run_troubleshooter(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
    recent_status: Option<String>,
    recent_logs: Option<Vec<String>>,
) -> Result<TroubleshooterReport, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session_fresh(state.clone()).await?;
    let auth_session = load_auth_session_fresh(state.clone()).await?;
    let readiness = diagnostics::build_launch_readiness(diagnostics::ReadinessContext {
        settings,
        atlas_session,
        auth_session,
        game_dir: game_dir.clone(),
    });
    let merged_logs = diagnostics::collect_troubleshooter_logs(game_dir.as_deref(), recent_logs);

    Ok(diagnostics::run_troubleshooter(
        diagnostics::TroubleshooterInput {
            readiness,
            recent_status,
            recent_logs: merged_logs,
        },
    ))
}

#[tauri::command]
pub async fn apply_fix(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    action: FixAction,
    game_dir: Option<String>,
    pack_id: Option<String>,
    channel: Option<String>,
) -> Result<FixResult, String> {
    let settings = load_settings(&state)?;
    if matches!(action, FixAction::SetSafeMemory) {
        let mut next = settings;
        let old = next.default_java_memory_mb;
        if next.default_java_memory_mb < 4096 {
            next.default_java_memory_mb = 4096;
        }
        settings::save_settings(&next)?;
        let mut guard = state
            .settings
            .lock()
            .map_err(|_| "Settings lock poisoned".to_string())?;
        *guard = next.clone();
        return Ok(FixResult {
            action,
            applied: old != next.default_java_memory_mb,
            message: format!("Default memory set to {} MB.", next.default_java_memory_mb),
        });
    }

    let atlas_session = load_atlas_session_fresh(state.clone()).await?;
    let resolved_pack_id =
        pack_id.or_else(|| diagnostics::infer_pack_id_for_game_dir(&settings, game_dir.as_deref()));

    diagnostics::apply_fix(
        &window,
        diagnostics::ApplyFixInput {
            action,
            settings,
            atlas_session,
            game_dir,
            pack_id: resolved_pack_id,
            channel,
        },
    )
    .await
}

#[tauri::command]
pub async fn repair_installation(
    window: tauri::Window,
    state: tauri::State<'_, AppState>,
    game_dir: String,
    pack_id: Option<String>,
    channel: Option<String>,
    preserve_saves: Option<bool>,
) -> Result<RepairResult, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session_fresh(state.clone()).await?;
    let resolved_pack_id =
        pack_id.or_else(|| diagnostics::infer_pack_id_for_game_dir(&settings, Some(&game_dir)));

    diagnostics::repair_installation(
        &window,
        diagnostics::RepairInput {
            settings,
            atlas_session,
            game_dir,
            pack_id: resolved_pack_id,
            channel,
            preserve_saves: preserve_saves.unwrap_or(true),
        },
    )
    .await
}

#[tauri::command]
pub async fn create_support_bundle(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
    recent_status: Option<String>,
    recent_logs: Option<Vec<String>>,
) -> Result<SupportBundleResult, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session_fresh(state.clone()).await?;
    let auth_session = load_auth_session_fresh(state.clone()).await?;
    let readiness = diagnostics::build_launch_readiness(diagnostics::ReadinessContext {
        settings,
        atlas_session,
        auth_session,
        game_dir: game_dir.clone(),
    });
    diagnostics::create_support_bundle(diagnostics::SupportBundleInput {
        readiness,
        game_dir,
        recent_status,
        recent_logs: recent_logs.unwrap_or_default(),
    })
}
