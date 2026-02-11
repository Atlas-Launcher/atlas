use runner_v2_rcon::load_rcon_settings;
use std::path::Path;
use tracing::warn;

/// Try to run save-off/save-all via RCON. Returns Ok(true) if save-off was applied and we should later
/// call save-on. Returns Ok(false) if RCON is not available or save-off was not applied.
pub async fn rcon_save_off(server_root: &Path) -> Result<bool, String> {
    let current = server_root.join("current");
    if let Ok(Some(settings)) = load_rcon_settings(&current).await {
        let client = runner_v2_rcon::RconClient::new(settings.address, settings.password);
        // save-all then save-off
        if let Err(err) = client.execute("save-all").await {
            warn!("rcon save-all failed: {}", err);
            return Ok(false);
        }
        if let Err(err) = client.execute("save-off").await {
            warn!("rcon save-off failed: {}", err);
            return Ok(false);
        }
        return Ok(true);
    }
    Ok(false)
}

pub async fn rcon_save_on(server_root: &Path) -> Result<(), String> {
    let current = server_root.join("current");
    if let Ok(Some(settings)) = load_rcon_settings(&current).await {
        let client = runner_v2_rcon::RconClient::new(settings.address, settings.password);
        if let Err(err) = client.execute("save-on").await {
            warn!("rcon save-on failed: {}", err);
            return Err(format!("rcon save-on failed: {}", err));
        }
    }
    Ok(())
}
