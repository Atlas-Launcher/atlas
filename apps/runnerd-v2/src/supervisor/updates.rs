use std::sync::Arc;

use atlas_client::hub::HubClient;
use atlas_client::sse::SseParser;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use runner_core_v2::proto::ServerStatus;
use runner_v2_rcon::{load_rcon_settings, RconClient};

use crate::config::DeployKeyConfig;

use super::server::{apply_pack_blob, spawn_server, stop_server_internal};
use super::state::SharedState;
use super::util::current_server_root;

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

    let mut hub = match HubClient::new(&config.hub_url) {
        Ok(value) => value,
        Err(err) => {
            warn!("failed to create hub client: {err}");
            return;
        }
    };
    hub.set_service_token(config.deploy_key.clone());
    let hub = Arc::new(hub);

    let whitelist_state = state.clone();
    let whitelist_hub = hub.clone();
    let whitelist_config = config.clone();
    tokio::spawn(async move {
        let mut backoff = Duration::from_secs(2);
        loop {
            match listen_whitelist_events(whitelist_hub.clone(), &whitelist_config, whitelist_state.clone()).await {
                Ok(()) => backoff = Duration::from_secs(2),
                Err(err) => warn!("whitelist stream error: {err}"),
            }
            sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    });

    let update_state = state.clone();
    let update_hub = hub.clone();
    tokio::spawn(async move {
        let mut backoff = Duration::from_secs(2);
        loop {
            match listen_pack_update_events(update_hub.clone(), &config, update_state.clone()).await {
                Ok(()) => backoff = Duration::from_secs(2),
                Err(err) => warn!("pack update stream error: {err}"),
            }
            sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    });
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WhitelistPushEvent {
    pack_id: String,
    #[serde(rename = "type")]
    event_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackUpdateEvent {
    pack_id: String,
    #[serde(rename = "type")]
    event_type: Option<String>,
    channel: Option<String>,
}

async fn listen_whitelist_events(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    let response = hub
        .open_whitelist_events(&config.pack_id)
        .await
        .map_err(|err| format!("whitelist SSE failed: {err}"))?;
    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("whitelist SSE read failed: {err}"))?;
        for payload in parser.push_chunk(&chunk) {
            if should_trigger_whitelist_sync(&payload, &config.pack_id) {
                if let Err(err) = sync_whitelist(hub.clone(), &config.pack_id, state.clone()).await {
                    warn!("whitelist sync failed: {err}");
                }
            }
        }
    }

    Err("whitelist stream ended".to_string())
}

fn should_trigger_whitelist_sync(payload: &str, pack_id: &str) -> bool {
    if payload.is_empty() {
        return false;
    }

    if let Ok(event) = serde_json::from_str::<WhitelistPushEvent>(payload) {
        return event.pack_id == pack_id && event.event_type.as_deref() != Some("ready");
    }

    false
}

async fn listen_pack_update_events(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    let response = hub
        .open_pack_update_events(&config.pack_id)
        .await
        .map_err(|err| format!("pack update SSE failed: {err}"))?;
    let mut stream = response.bytes_stream();
    let mut parser = SseParser::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("pack update SSE read failed: {err}"))?;
        for payload in parser.push_chunk(&chunk) {
            if should_trigger_pack_update(&payload, &config.pack_id, &config.channel) {
                if let Err(err) = apply_pack_update(hub.clone(), config, state.clone()).await {
                    warn!("pack update failed: {err}");
                }
            }
        }
    }

    Err("pack update stream ended".to_string())
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
    let players = hub
        .get_whitelist(pack_id)
        .await
        .map_err(|err| format!("whitelist fetch failed: {err}"))?;
    let whitelist_data = players
        .into_iter()
        .map(|player| {
            serde_json::json!({
                "name": player.name,
                "uuid": player.uuid,
            })
        })
        .collect::<Vec<_>>();

    let path = server_root.join("current").join("whitelist.json");
    let content = serde_json::to_string_pretty(&whitelist_data)
        .map_err(|err| format!("whitelist serialize failed: {err}"))?;
    let previous = tokio::fs::read_to_string(&path).await.ok();
    if previous.as_deref() == Some(content.as_str()) {
        return Ok(());
    }

    tokio::fs::write(&path, content)
        .await
        .map_err(|err| format!("whitelist write failed: {err}"))?;

    if let Ok(Some(settings)) = load_rcon_settings(&server_root.join("current")).await {
        let rcon = RconClient::new(settings.address, settings.password);
        let _ = rcon.execute("whitelist reload").await;
    }

    Ok(())
}

async fn apply_pack_update(
    hub: Arc<HubClient>,
    config: &DeployKeyConfig,
    state: SharedState,
) -> Result<(), String> {
    info!("pack update detected; applying update");
    stop_server_internal(state.clone(), false)
        .await
        .map_err(|err| format!("failed to stop server: {}", err.message))?;

    let build = hub
        .get_build_blob(&config.pack_id, &config.channel)
        .await
        .map_err(|err| format!("download build failed: {err}"))?;

    let server_root = current_server_root(&state)
        .await
        .ok_or_else(|| "server root not configured".to_string())?;
    let launch_plan = apply_pack_blob(&server_root, &build.bytes)
        .await
        .map_err(|err| err.message)?;

    let env = {
        let guard = state.lock().await;
        guard.env.clone()
    };

    let logs = {
        let guard = state.lock().await;
        guard.logs.clone()
    };
    let child = spawn_server(&launch_plan, &server_root, &env, logs)
        .await
        .map_err(|err| format!("failed to start server: {err}"))?;
    let pid = child.id().unwrap_or_default() as i32;
    let started_at_ms = super::util::now_millis();

    let mut guard = state.lock().await;
    guard.child = Some(child);
    guard.launch_plan = Some(launch_plan);
    guard.restart_attempts = 0;
    guard.last_start_ms = Some(started_at_ms);
    let profile = guard.profile.clone().unwrap_or_else(|| "default".into());
    guard.status = ServerStatus::Running {
        profile,
        pid,
        started_at_ms,
        meta: Default::default(),
    };

    Ok(())
}
