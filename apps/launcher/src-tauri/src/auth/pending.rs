use crate::paths::{auth_store_dir, ensure_dir, file_exists};
use super::error::AuthError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAuth {
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
}

fn pending_auth_path() -> Result<PathBuf, AuthError> {
    Ok(auth_store_dir()?.join("pending_auth.json"))
}

pub fn load_pending_auth() -> Result<Option<PendingAuth>, AuthError> {
    let path = pending_auth_path()?;
    if !file_exists(&path) {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read pending auth: {err}"))?;
    let pending = serde_json::from_slice::<PendingAuth>(&bytes)
        .map_err(|err| format!("Failed to parse pending auth: {err}"))?;
    Ok(Some(pending))
}

pub fn save_pending_auth(pending: &PendingAuth) -> Result<(), AuthError> {
    let path = pending_auth_path()?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_vec_pretty(pending)
        .map_err(|err| format!("Failed to serialize pending auth: {err}"))?;
    fs::write(&path, payload).map_err(|err| format!("Failed to write pending auth: {err}"))?;
    Ok(())
}

pub fn clear_pending_auth() -> Result<(), AuthError> {
    let path = pending_auth_path()?;
    if file_exists(&path) {
        fs::remove_file(&path).map_err(|err| format!("Failed to remove pending auth: {err}"))?;
    }
    Ok(())
}
