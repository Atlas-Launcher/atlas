use super::error::AuthError;
use crate::paths::{auth_store_dir, ensure_dir, file_exists};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasPendingAuth {
    pub auth_base_url: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_verifier: String,
}

fn pending_auth_path() -> Result<PathBuf, AuthError> {
    Ok(auth_store_dir()?.join("pending_atlas_auth.json"))
}

pub fn load_pending_atlas_auth() -> Result<Option<AtlasPendingAuth>, AuthError> {
    let path = pending_auth_path()?;
    if !file_exists(&path) {
        return Ok(None);
    }
    let bytes =
        fs::read(&path).map_err(|err| format!("Failed to read pending Atlas auth: {err}"))?;
    let pending = serde_json::from_slice::<AtlasPendingAuth>(&bytes)
        .map_err(|err| format!("Failed to parse pending Atlas auth: {err}"))?;
    Ok(Some(pending))
}

pub fn save_pending_atlas_auth(pending: &AtlasPendingAuth) -> Result<(), AuthError> {
    let path = pending_auth_path()?;
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_vec_pretty(pending)
        .map_err(|err| format!("Failed to serialize pending Atlas auth: {err}"))?;
    fs::write(&path, payload)
        .map_err(|err| format!("Failed to write pending Atlas auth: {err}"))?;
    Ok(())
}

pub fn clear_pending_atlas_auth() -> Result<(), AuthError> {
    let path = pending_auth_path()?;
    if file_exists(&path) {
        fs::remove_file(&path)
            .map_err(|err| format!("Failed to remove pending Atlas auth: {err}"))?;
    }
    Ok(())
}
