use std::path::PathBuf;
use std::sync::Arc;

use atlas_client::hub::HubClient;
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, debug, error};

use runner_v2_rcon::{load_rcon_settings, RconClient};

use crate::config::DeployKeyConfig;

use super::server::{start_server, stop_server_internal};
use super::state::SharedState;
use super::util::current_server_root;

const POLL_INTERVAL_SECS: u64 = 60;
const PACK_ETAG_FILENAME: &str = ".runner/pack_etag.txt";
const WHITELIST_ETAG_FILENAME: &str = ".runner/whitelist_etag.txt";

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
    let hub_url = "https://atlas.nathanm.org";
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
        if let Ok(etag) = read_whitelist_etag_from_disk(&server_root).await {
            if !etag.is_empty() {
                let mut guard = state.lock().await;
                guard.whitelist_etag = Some(etag);
                info!("loaded persisted whitelist etag from disk");
            }
        }
    }

    // create watcher stop and done flags and persist into shared state so other code can signal shutdown and observe completion
    let watcher_stop_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let watcher_done_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut guard = state.lock().await;
        guard.watcher_stop = Some(watcher_stop_flag.clone());
        guard.watcher_done = Some(watcher_done_flag.clone());
    }

    // Spawn supervisor as an async task on the current runtime; this avoids cross-runtime deadlocks
    // and keeps all async locking on the same runtime. We store watcher flags in state above
    // (so other code can signal/observe) and don't keep the task JoinHandle in state.
    let supervisor_state = state.clone();
    let _ = tokio::spawn(async move {
        let mut failures: u32 = 0;
        loop {
            watcher_done_flag.store(false, std::sync::atomic::Ordering::Relaxed);

            let worker_stop = watcher_stop_flag.clone();
            let worker_done = watcher_done_flag.clone();
            let whub = hub_url.clone();
            let wdeploy = hub_deploy_key.clone();
            let w_whitelist_cfg = config.clone();
            let w_update_cfg = config.clone();
            let w_state_whitelist = supervisor_state.clone();
            let w_state_update = supervisor_state.clone();

            let start = std::time::Instant::now();

            // Spawn worker as a tokio task; worker runs two loops concurrently and returns when they exit.
            let worker_handle = tokio::spawn(async move {
                // Whitelist loop
                let whitelist_fut = async {
                    let poll_interval = Duration::from_secs(POLL_INTERVAL_SECS);
                    loop {
                        if worker_stop.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("watcher worker: whitelist loop exiting due to stop request");
                            break;
                        }

                        match HubClient::new(&whub) {
                            Ok(mut h) => {
                                h.set_service_token(wdeploy.clone());
                                let h = Arc::new(h);
                                if let Err(err) = poll_whitelist(h, &w_whitelist_cfg, w_state_whitelist.clone()).await {
                                    warn!("whitelist poll error: {err}");
                                }
                            }
                            Err(err) => {
                                warn!("watcher worker: failed to create hub client for whitelist: {err}");
                                sleep(Duration::from_secs(5)).await;
                                continue;
                            }
                        }

                        sleep(poll_interval).await;
                    }
                };

                // Update loop
                let update_fut = async {
                    let poll_interval = Duration::from_secs(POLL_INTERVAL_SECS);
                    loop {
                        if worker_stop.load(std::sync::atomic::Ordering::Relaxed) {
                            info!("watcher worker: update loop exiting due to stop request");
                            break;
                        }

                        match HubClient::new(&whub) {
                            Ok(mut h) => {
                                h.set_service_token(wdeploy.clone());
                                let h = Arc::new(h);
                                if let Err(err) = poll_pack_update(h, &w_update_cfg, w_state_update.clone()).await {
                                    warn!("pack update poll error: {err}");
                                }
                            }
                            Err(err) => {
                                warn!("watcher worker: failed to create hub client for updates: {err}");
                                sleep(Duration::from_secs(5)).await;
                                continue;
                            }
                        }

                        sleep(poll_interval).await;
                    }
                };

                tokio::join!(whitelist_fut, update_fut);
                worker_done.store(true, std::sync::atomic::Ordering::Relaxed);
            });

            // Wait for worker to finish and measure runtime
            let _ = worker_handle.await;
            watcher_done_flag.store(true, std::sync::atomic::Ordering::Relaxed);

            let elapsed_ms = start.elapsed().as_millis() as u64;
            // If the worker ran for a reasonable time, treat this as non-failure and reset backoff;
            // otherwise increment the failure counter (used to compute exponential backoff below).
            if elapsed_ms >= 10_000 {
                failures = 0;
            } else {
                failures = failures.saturating_add(1);
            }

            if watcher_stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                info!("watcher supervisor: stop requested, exiting");
                break;
            }

            let backoff_ms = std::cmp::min(30_000, 1_000u64.saturating_mul(2u64.saturating_pow(std::cmp::min(failures, 10) as u32)));
            if failures > 5 {
                warn!("watcher supervisor: worker exited unexpectedly {} times; backing off for {}ms", failures, backoff_ms);
            } else {
                warn!("watcher supervisor: worker exited unexpectedly, restarting in {}ms", backoff_ms);
            }

            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    });

    info!("started watcher supervisor task");
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
                // update in-memory state while holding lock, then drop before doing I/O
                {
                    let mut guard = state.lock().await;
                    guard.pack_etag = Some(normalized.clone());
                }
                debug!("pack metadata not modified; stored etag={}", normalized);

                // Persist to disk (do not hold state lock while awaiting)
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
                // set in-memory state then persist without holding the lock
                {
                    let mut guard = state.lock().await;
                    guard.pack_etag = Some(normalized.clone());
                }
                debug!("pack metadata returned new etag={}", normalized);

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
            // set in-memory state then persist without holding the lock
            {
                let mut guard = state.lock().await;
                guard.whitelist_etag = Some(normalized.clone());
            }
            debug!("whitelist not modified; stored etag={}", etag);

            if let Some(_parent) = server_root.join(".runner").parent() {
                let _ = write_whitelist_etag_to_disk(server_root, &normalized).await;
            }
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
            // update in-memory state then persist without holding lock
            {
                let mut guard = state.lock().await;
                guard.whitelist_etag = Some(normalized.clone());
            }
            let _ = write_whitelist_etag_to_disk(server_root, &normalized).await;
        }
        return Ok(());
    }

    tokio::fs::write(&path, content)
        .await
        .map_err(|err| format!("whitelist write failed: {err}"))?;

    // Persist etag after successful write: normalize, set in-memory, then persist to disk
    if !etag.is_empty() {
        let normalized = normalize_etag_value(&etag);
        {
            let mut guard = state.lock().await;
            guard.whitelist_etag = Some(normalized.clone());
        }
        let _ = write_whitelist_etag_to_disk(server_root, &normalized).await;
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

    // Obtain lifecycle lock so stop/start/update do not run concurrently
    // Use a short timeout strategy: attempt to acquire the lock, but if another lifecycle op
    // is running, back off and fail the update for now.
    let lock = {
        let guard = state.lock().await;
        guard.lifecycle_lock.clone()
    };

    // Try to acquire without awaiting indefinitely: try_lock is not available on tokio::sync::Mutex,
    // so we'll do a try with timeout. Bind the guard so the lock is held across awaits.
    let lifecycle_guard = match tokio::time::timeout(Duration::from_secs(5), lock.lock()).await {
        Ok(g) => g,
        Err(_) => {
            return Err("another lifecycle operation is in progress; try again later".into());
        }
    };

    // Stop the server (graceful) before applying the update
    if let Err(err) = stop_server_internal(state.clone(), false).await {
        return Err(format!("failed to stop server: {}", err.message));
    }

    let server_root = current_server_root(&state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;

    // Use the higher-level start_server path to apply the pack and start the server.
    let profile = {
        let guard = state.lock().await;
        guard.profile.clone().unwrap_or_else(|| "default".into())
    };

    // If the build requests a force reinstall (or requires a full reinstall), clear the
    // active `current` directory so the provisioner does not preserve or merge old files.
    // We move `current` into `.runner/backup/current-<ms>` so a backup is retained.
    if build.force_reinstall || build.requires_full_reinstall {
        info!("build requests force reinstall; clearing existing server data before apply");
        let current = server_root.join("current");
        match tokio::fs::try_exists(&current).await {
            Ok(true) => {
                let backup_dir = server_root.join(".runner").join("backup");
                if let Err(err) = tokio::fs::create_dir_all(&backup_dir).await {
                    warn!("failed to create backup dir {}: {}", backup_dir.display(), err);
                }
                let backup = backup_dir.join(format!("current-{}", super::util::now_millis()));
                match tokio::fs::rename(&current, &backup).await {
                    Ok(_) => info!("moved existing current to backup: {}", backup.display()),
                    Err(err) => {
                        warn!("failed to move current to backup: {}", err);
                        // As a last resort try to remove the directory so the installer starts clean
                        if let Err(err2) = tokio::fs::remove_dir_all(&current).await {
                            return Err(format!("failed to clear existing server directory: {}, {}", err, err2));
                        } else {
                            info!("removed existing current directory");
                        }
                    }
                }
            }
            Ok(false) => {
                debug!("no existing current directory to clear");
            }
            Err(err) => {
                warn!("failed to probe existing current directory: {}", err);
            }
        }
    }

    start_server(profile.clone(), &build.bytes, server_root.clone(), state.clone())
        .await
        .map_err(|err| format!("failed to start server: {}", err.message))?;

    // start_server sets child/launch_plan/status; poll_pack_update will persist the
    // applied build id into state after successful apply. We avoid resetting
    // current_pack_build_id here which could cause transient `None` and confusing comparisons.

    // Drop lifecycle_guard to release the lifecycle mutex now that start is complete.
    // (lifecycle_guard was bound earlier)
    drop(lifecycle_guard);

    Ok(())
}

// Helper: normalize an ETag string to its bare token (strip surrounding quotes)
fn normalize_etag_value(s: &str) -> String {
    let s = s.trim();
    // Remove weak prefix if present
    let s = if let Some(stripped) = s.strip_prefix("W/") {
        stripped.trim()
    } else {
        s
    };
    // Remove surrounding double quotes if present
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len()-1].to_string()
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

// Whitelist etag helpers
async fn read_whitelist_etag_from_disk(server_root: &PathBuf) -> Result<String, String> {
    let path = server_root.join(WHITELIST_ETAG_FILENAME);
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => Ok(content.trim().to_string()),
        Err(_) => Ok(String::new()),
    }
}

async fn write_whitelist_etag_to_disk(server_root: &PathBuf, etag: &str) -> Result<(), String> {
    let path = server_root.join(WHITELIST_ETAG_FILENAME);
    if let Some(parent) = path.parent() {
        if let Err(err) = tokio::fs::create_dir_all(parent).await {
            return Err(format!("failed to create .runner directory: {err}"));
        }
    }
    tokio::fs::write(&path, etag)
        .await
        .map_err(|err| format!("failed to write whitelist etag file: {err}"))
}
