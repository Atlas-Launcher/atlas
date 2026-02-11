use runner_v2_rcon::{RconClient, load_rcon_settings};

use super::state::SharedState;
use super::util::current_server_root;

pub async fn execute_rcon_command(state: &SharedState, command: &str) -> Result<String, String> {
    let server_root = current_server_root(state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;
    let settings = load_rcon_settings(&server_root.join("current"))
        .await
        .map_err(|err| format!("failed to load rcon settings: {err}"))?
        .ok_or_else(|| "RCON not configured".to_string())?;
    let rcon = RconClient::new(settings.address, settings.password);
    rcon.execute(command)
        .await
        .map_err(|err| format!("RCON failed: {err}"))
}

pub async fn ensure_rcon_available(state: &SharedState) -> Result<(), String> {
    let server_root = current_server_root(state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;
    let settings = load_rcon_settings(&server_root.join("current"))
        .await
        .map_err(|err| format!("failed to load rcon settings: {err}"))?;
    if settings.is_none() {
        return Err("RCON not configured".to_string());
    }
    Ok(())
}
