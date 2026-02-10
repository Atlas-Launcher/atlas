use std::path::PathBuf;
use std::sync::Arc;

use atlas_client::hub::HubClient;
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, debug};

use runner_v2_rcon::{load_rcon_settings, RconClient};

use crate::config::DeployKeyConfig;

use super::server::{start_server, stop_server_internal};
use super::state::SharedState;
use super::util::current_server_root;

const POLL_INTERVAL_SECS: u64 = 60;
const PACK_ETAG_FILENAME: &str = ".runner/pack_etag.txt";

pub async fn ensure_watchers(state: SharedState) {
    let start_watchers = {
        let mut guard = state.lock().await;
        if guard.watchers_started {
            false
        } else {
            guard.watchers_started = true;
            true
        }
    };

    if !start_watchers {
        return;
    }

    let config = match crate::config::load_deploy_key() {
        Ok(Some(value)) => value,
        Ok(None) => {
            warn!("deploy key not configured; skipping update watchers");
            return;
        }
        Err(err) => {
            warn!("failed to load deploy key config: {err}");
            return;
        }
    };

    // We'll create HubClient instances inside each watcher task instead of capturing one here.
    let hub_url = config.hub_url.clone();
    let hub_deploy_key = config.deploy_key.clone();

    // Try to load persisted pack etag from disk (if present). This allows the watcher to send If-None-Match
    // on the very first poll instead of always sending None.
    if let Some(server_root) = current_server_root(&state).await {
        if let Ok(etag) = read_pack_etag_from_disk(&server_root).await {
            if !etag.is_empty() {
                let mut guard = state.lock().await;
                guard.pack_etag = Some(etag);
                info!("loaded persisted pack etag from disk");
            }
        }
    }

    // Start a single dedicated watcher worker thread which runs a current-thread tokio runtime
    // and executes both the whitelist and pack-update loops concurrently. This keeps all
    // watcher logic in one place and avoids requiring Send for internal non-Send futures.
    let worker_state_whitelist = state.clone();
    let worker_state_update = state.clone();
    let worker_whitelist_config = config.clone();
    let worker_update_config = config.clone();
    let worker_hub_url = hub_url.clone();
    let worker_deploy_key = hub_deploy_key.clone();

    // create watcher stop and done flags and persist into shared state so other code can signal shutdown and observe completion
    let watcher_stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watcher_done_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut guard = state.lock().await;
        guard.watcher_stop = Some(watcher_stop_flag.clone());
        guard.watcher_done = Some(watcher_done_flag.clone());
    }

    let handle = std::thread::Builder::new()
        .name("atlas-watcher-worker".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create watcher worker runtime");

            rt.block_on(async move {
                // Whitelist loop future
                let whitelist_fut = async {
                    // construct local HubClient for this task
                    let mut local_hub = match HubClient::new(&worker_hub_url) {
                        Ok(h) => h,
                        Err(err) => {
                            warn!("watcher worker: failed to create hub client for whitelist: {err}");
                            return;
                        }
                    };
                    local_hub.set_service_token(worker_deploy_key.clone());
                    let local_hub = Arc::new(local_hub);

                    let poll_interval = Duration::from_secs(POLL_INTERVAL_SECS); // 1 minute
                    loop {
                        if watcher_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("watcher worker: whitelist loop exiting due to stop request");
                            break;
                        }
                        match poll_whitelist(local_hub.clone(), &worker_whitelist_config, worker_state_whitelist.clone()).await {
                            Ok(()) => {},
                            Err(err) => warn!("whitelist poll error: {err}"),
                        }
                        sleep(poll_interval).await;
                    }
                };

                // Pack update loop future
                let update_fut = async {
                    // construct local HubClient for this task
                    let mut local_hub = match HubClient::new(&worker_hub_url) {
                        Ok(h) => h,
                        Err(err) => {
                            warn!("watcher worker: failed to create hub client for updates: {err}");
                            return;
                        }
                    };
                    local_hub.set_service_token(worker_deploy_key.clone());
                    let local_hub = Arc::new(local_hub);

                    let poll_interval = Duration::from_secs(POLL_INTERVAL_SECS); // 5 minutes
                    loop {
                        if watcher_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("watcher worker: update loop exiting due to stop request");
                            break;
                        }
                        match poll_pack_update(local_hub.clone(), &worker_update_config, worker_state_update.clone()).await {
                            Ok(()) => {},
                            Err(err) => warn!("pack update poll error: {err}"),
                        }
                        sleep(poll_interval).await;
                    }
                };

                // Run both loops concurrently; they are infinite loops and will run until process exit.
                tokio::join!(whitelist_fut, update_fut);

                // mark done before returning from the thread runtime
                watcher_done_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            });
        })
        .expect("failed to spawn watcher worker thread");

    // Store the handle so the daemon can wait for it during shutdown
    {
        let mut guard = state.lock().await;
        guard.watcher_handle = Some(handle);
    }

    info!("started watcher worker thread");
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackUpdateEvent {
    pack_id: String,
    #[serde(rename = "type")]
    event_type: Option<String>,
    channel: Option<String>,
}

async fn poll_whitelist(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    // Always sync whitelist on poll
    sync_whitelist(hub, &config.pack_id, state).await
}

async fn poll_pack_update(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    // First check pack metadata with ETag caching
    let current_etag = {
        let guard = state.lock().await;
        guard.pack_etag.clone()
    };

    // Send the stored ETag as a quoted If-None-Match header (server expects quoted values).
    let current_etag_header = current_etag.as_ref().map(|tok| format!("\"{}\"", tok));
    debug!("poll_pack_update: sending If-None-Match={:?}", current_etag_header);

    match hub
        .get_pack_metadata_with_etag(&config.pack_id, &config.channel, current_etag_header.as_deref())
        .await
    {
        Ok((None, returned_etag)) => {
            // Not modified. Persist the returned etag (normalized) so future requests send If-None-Match.
            if !returned_etag.is_empty() {
                let normalized = normalize_etag_value(&returned_etag);
                let mut guard = state.lock().await;
                guard.pack_etag = Some(normalized.clone());
                debug!("pack metadata not modified; stored etag={}", normalized);

                // Persist to disk
                if let Some(server_root) = current_server_root(&state).await {
                    let _ = write_pack_etag_to_disk(&server_root, &normalized).await;
                }
            } else {
                debug!("pack metadata not modified; no ETag returned");
            }
            return Ok(());
        }
        Ok((Some(metadata), new_etag)) => {
            // Always persist the etag (if non-empty)
            if !new_etag.is_empty() {
                let normalized = normalize_etag_value(&new_etag);
                let mut guard = state.lock().await;
                guard.pack_etag = Some(normalized.clone());
                debug!("pack metadata returned new etag={}", normalized);

                // Persist to disk
                if let Some(server_root) = current_server_root(&state).await {
                    let _ = write_pack_etag_to_disk(&server_root, &normalized).await;
                }
            }

            // IMPORTANT: only apply update if build actually differs
            let current_build_id = {
                let guard = state.lock().await;
                guard.current_pack_build_id.clone()
            };

            if current_build_id.as_deref() == Some(metadata.build_id.as_str()) {
                // metadata changed / revalidated, but build didn't
                debug!(
                "pack metadata refreshed; build unchanged (build: {})",
                metadata.build_id
            );
                return Ok(());
            }

            info!(
            "pack update detected; applying update (build: {})",
            metadata.build_id
        );

            // proceed with update: capture the build id so we can update shared state on success
            let target_build_id = metadata.build_id.clone();
            // Apply the pack update
            if let Err(err) = apply_pack_update(hub, config, state.clone()).await {
                return Err(err);
            }

            // On successful update, persist current_pack_build_id in state
            {
                let mut guard = state.lock().await;
                guard.current_pack_build_id = Some(target_build_id);
            }
            return Ok(());
        }
        Err(err) => {
            warn!("pack metadata check failed: {err}, falling back to direct update");
        }
    }


    // Apply the pack update
    apply_pack_update(hub, config, state).await
}

fn should_trigger_pack_update(payload: &str, pack_id: &str, channel: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(event) = serde_json::from_str::<PackUpdateEvent>(payload) {
        let channel_matches = event
            .channel
            .as_deref()
            .map(|value| value.eq_ignore_ascii_case(channel))
            .unwrap_or(true);
        return event.pack_id == pack_id
            && channel_matches
            && event.event_type.as_deref() != Some("ready");
    }

    false
}

async fn sync_whitelist(
    hub: Arc<HubClient>,
    pack_id: &str,
    state: SharedState,
) -> Result<(), String> {
    let server_root = current_server_root(&state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;
    sync_whitelist_to_root(hub, pack_id, &server_root, state).await
}

pub(crate) async fn sync_whitelist_to_root(
    hub: Arc<HubClient>,
    pack_id: &str,
    server_root: &PathBuf,
    state: SharedState,
) -> Result<(), String> {
    // Try to use ETag-aware endpoint to avoid unnecessary writes
    let current_etag = {
        let guard = state.lock().await;
        guard.whitelist_etag.clone()
    };

    // Quote the bare token when sending If-None-Match to match server expectation
    let current_etag_header = current_etag.as_ref().map(|tok| format!("\"{}\"", tok));

    let (players, etag) = hub
        .get_whitelist_with_version(pack_id, current_etag_header.as_deref())
        .await
        .map_err(|err| format!("whitelist fetch failed: {err}"))?;

    // If the server returned 304 (represented by empty players vec), skip writing and persist etag
    if players.is_empty() {
        if !etag.is_empty() {
            let normalized = normalize_etag_value(&etag);
            let mut guard = state.lock().await;
            guard.whitelist_etag = Some(normalized.clone());
            debug!("whitelist not modified; stored etag={}", etag);

            // Persist whitelist etag to disk alongside pack etag so restarts keep both
            let _ = write_pack_etag_to_disk(server_root, &normalized).await;
        } else {
            debug!("whitelist not modified; no ETag returned");
        }
        return Ok(());
    }

    let whitelist_data = players
        .iter()
        .map(|player| {
            serde_json::json!({
                "name": player.name,
                "uuid": format_uuid_with_dashes(&player.uuid),
            })
        })
        .collect::<Vec<_>>();

    let content = serde_json::to_string_pretty(&whitelist_data)
        .map_err(|err| format!("whitelist serialize failed: {err}"))?;

    let path = server_root.join("current").join("whitelist.json");
    let previous = tokio::fs::read_to_string(&path).await.ok();
    if previous.as_deref() == Some(content.as_str()) {
        // Persist the returned etag even if content matches
        if !etag.is_empty() {
            let normalized = normalize_etag_value(&etag);
            let mut guard = state.lock().await;
            guard.whitelist_etag = Some(normalized);
        }
        return Ok(());
    }

    tokio::fs::write(&path, content)
        .await
        .map_err(|err| format!("whitelist write failed: {err}"))?;

    // Persist etag after successful write
    {
        let mut guard = state.lock().await;
        if !etag.is_empty() {
            guard.whitelist_etag = Some(etag);
        }
    }

    if let Ok(Some(settings)) = load_rcon_settings(&server_root.join("current")).await {
        let rcon = RconClient::new(settings.address, settings.password);
        let _ = rcon.execute("whitelist reload").await;
    }

    Ok(())
}

fn format_uuid_with_dashes(value: &str) -> String {
    let compact: String = value.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
    if compact.len() != 32 {
        return value.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &compact[0..8],
        &compact[8..12],
        &compact[12..16],
        &compact[16..20],
        &compact[20..32]
    )
}

async fn apply_pack_update(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    info!("pack update detected; applying update");

    // Download the build first so we don't stop the running server unless the download succeeds.
    let build = hub
        .get_build_blob(&config.pack_id, &config.channel)
        .await
        .map_err(|err| format!("download build failed: {err}"))?;

    // Stop the server (graceful) before applying the update
    stop_server_internal(state.clone(), false)
        .await
        .map_err(|err| format!("failed to stop server: {}", err.message))?;

    let server_root = current_server_root(&state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;

    // Use the higher-level start_server path to apply the pack and start the server.
    let profile = {
        let guard = state.lock().await;
        guard.profile.clone().unwrap_or_else(|| "default".into())
    };

    start_server(profile.clone(), &build.bytes, server_root.clone(), state.clone())
        .await
        .map_err(|err| format!("failed to start server: {}", err.message))?;

    // start_server sets child/launch_plan/status; refresh current build id from disk if present
    let mut guard = state.lock().await;
    guard.current_pack_build_id = None; // reset current build id

    // Try to read the current build id from disk (if present) and persist into shared state
    let build_id_path = server_root.join("current").join(".runner").join("build_id.txt");
    if let Ok(content) = tokio::fs::read_to_string(&build_id_path).await {
        let id = content.trim().to_string();
        guard.current_pack_build_id = Some(id);
    }

    Ok(())
}

// Helper: normalize an ETag string to its bare token (strip surrounding quotes)
fn normalize_etag_value(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && ((s.starts_with("\"") && s.ends_with("\"")) || (s.starts_with("W/") && s.len() >= 3)) {
        // remove weak prefix and quotes if present
        let mut v = s.to_string();
        if v.starts_with("W/") {
            v = v[2..].to_string();
            v = v.trim().to_string();
        }
        if v.starts_with("\"") && v.ends_with("\"") && v.len() >= 2 {
            v = v[1..v.len()-1].to_string();
        }
        v
    } else {
        s.to_string()
    }
}

// Read persisted pack etag from disk for a given server root. Returns Ok(String) with empty string if missing.
async fn read_pack_etag_from_disk(server_root: &PathBuf) -> Result<String, String> {
    let path = server_root.join(PACK_ETAG_FILENAME);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(content.trim().to_string()),
        Err(_) => Ok(String::new()),
    }
}

// Write the given pack etag token to disk (creates parent .runner dir if necessary). Returns error string on failure.
async fn write_pack_etag_to_disk(server_root: &PathBuf, etag: &str) -> Result<(), String> {
    let path = server_root.join(PACK_ETAG_FILENAME);
    if let Some(parent) = path.parent() {
        if let Err(err) = tokio::fs::create_dir_all(parent).await {
            return Err(format!("failed to create .runner directory: {err}"));
        }
    }
    tokio::fs::write(&path, etag)
        .await
        .map_err(|err| format!("failed to write pack etag file: {err}"))
}
