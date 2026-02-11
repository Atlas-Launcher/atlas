use std::path::PathBuf;

use runner_v2_utils::runtime_paths_v2;

use super::state::SharedState;

pub fn default_server_root(profile: &str) -> PathBuf {
    let paths = runtime_paths_v2();
    paths.runtime_dir.join("servers").join(profile)
}

pub fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub async fn current_server_root(state: &SharedState) -> Option<PathBuf> {
    let guard = state.lock().await;
    guard.server_root.clone().or_else(|| {
        guard
            .profile
            .as_ref()
            .map(|profile| default_server_root(profile))
    })
}
