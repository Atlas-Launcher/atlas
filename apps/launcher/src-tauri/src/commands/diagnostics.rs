use crate::auth;
use crate::diagnostics;
use crate::models::{
    AtlasSession, AuthSession, FixAction, FixResult, LaunchReadinessReport, RepairResult,
    SupportBundleResult, TroubleshooterReport,
};
use crate::settings;
use crate::state::AppState;

fn load_auth_session(state: &tauri::State<'_, AppState>) -> Result<Option<AuthSession>, String> {
    let from_state = state
        .auth
        .lock()
        .map_err(|_| "Auth state lock poisoned".to_string())?
        .clone();
    if from_state.is_some() {
        return Ok(from_state);
    }
    auth::load_session().map_err(|err| err.to_string())
}

fn load_atlas_session(state: &tauri::State<'_, AppState>) -> Result<Option<AtlasSession>, String> {
    let from_state = state
        .atlas_auth
        .lock()
        .map_err(|_| "Atlas auth state lock poisoned".to_string())?
        .clone();
    if from_state.is_some() {
        return Ok(from_state);
    }
    auth::load_atlas_session().map_err(|err| err.to_string())
}

fn load_settings(state: &tauri::State<'_, AppState>) -> Result<crate::models::AppSettings, String> {
    state
        .settings
        .lock()
        .map_err(|_| "Settings lock poisoned".to_string())
        .map(|guard| guard.clone())
}

#[tauri::command]
pub fn get_launch_readiness(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
) -> Result<LaunchReadinessReport, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session(&state)?;
    let auth_session = load_auth_session(&state)?;
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
pub fn run_troubleshooter(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
    recent_status: Option<String>,
    recent_logs: Option<Vec<String>>,
) -> Result<TroubleshooterReport, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session(&state)?;
    let auth_session = load_auth_session(&state)?;
    let readiness = diagnostics::build_launch_readiness(diagnostics::ReadinessContext {
        settings,
        atlas_session,
        auth_session,
        game_dir,
    });

    Ok(diagnostics::run_troubleshooter(
        diagnostics::TroubleshooterInput {
            readiness,
            recent_status,
            recent_logs: recent_logs.unwrap_or_default(),
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

    let atlas_session = load_atlas_session(&state)?;
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
    let atlas_session = load_atlas_session(&state)?;
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
pub fn create_support_bundle(
    state: tauri::State<'_, AppState>,
    game_dir: Option<String>,
    recent_status: Option<String>,
    recent_logs: Option<Vec<String>>,
) -> Result<SupportBundleResult, String> {
    let settings = load_settings(&state)?;
    let atlas_session = load_atlas_session(&state)?;
    let auth_session = load_auth_session(&state)?;
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
