use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use super::server::spawn_server;
use super::state::SharedState;
use super::util::{default_server_root, now_millis};
use runner_core_v2::proto::{ExitInfo, ServerStatus};

pub async fn ensure_monitor(state: SharedState) {
    let start_monitor = {
        let mut guard = state.lock().await;
        if guard.monitor_started {
            false
        } else {
            guard.monitor_started = true;
            true
        }
    };

    if !start_monitor {
        return;
    }

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(1)).await;

            let (profile, server_root, env, launch_plan, restart_disabled, attempts) = {
                let mut guard = state.lock().await;
                if let Some(child) = guard.child.as_mut() {
                    if let Ok(Some(status)) = child.try_wait() {
                        let uptime_ms = guard
                            .last_start_ms
                            .map(|start| now_millis().saturating_sub(start))
                            .unwrap_or(0);
                        if uptime_ms >= RESET_AFTER_MS {
                            guard.restart_attempts = 0;
                        }
                        let exit = ExitInfo { code: status.code(), signal: None };
                        let profile = guard.profile.clone().unwrap_or_else(|| "default".into());
                        guard.child = None;
                        guard.status = ServerStatus::Exited {
                            profile: profile.clone(),
                            exit,
                            at_ms: now_millis(),
                        };
                        (
                            guard.profile.clone(),
                            guard.server_root.clone(),
                            guard.env.clone(),
                            guard.launch_plan.clone(),
                            guard.restart_disabled,
                            guard.restart_attempts,
                        )
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            };

            if restart_disabled {
                continue;
            }

            if attempts >= 5 {
                warn!("server crashed too many times, awaiting manual restart");
                continue;
            }

            let profile = profile.unwrap_or_else(|| "default".into());
            let server_root = match server_root {
                Some(value) => value,
                None => default_server_root(&profile),
            };
            let launch_plan = match launch_plan {
                Some(value) => value,
                None => {
                    warn!("missing launch plan, skipping restart");
                    continue;
                }
            };

            let backoff = restart_backoff(attempts);
            info!("server exited; restarting in {:?}", backoff);
            sleep(backoff).await;

            let logs = {
                let guard = state.lock().await;
                guard.logs.clone()
            };
            match spawn_server(&launch_plan, &server_root, &env, logs).await {
                Ok(child) => {
                    let pid = child.id().unwrap_or_default() as i32;
                    let started_at_ms = now_millis();
                    let mut guard = state.lock().await;
                    guard.child = Some(child);
                    guard.last_start_ms = Some(started_at_ms);
                    guard.status = ServerStatus::Running {
                        profile: profile.clone(),
                        pid,
                        started_at_ms,
                        meta: Default::default(),
                    };
                    guard.restart_attempts += 1;
                }
                Err(err) => {
                    warn!("restart failed: {err}");
                    let mut guard = state.lock().await;
                    guard.restart_attempts += 1;
                }
            }
        }
    });
}

fn restart_backoff(attempts: u32) -> Duration {
    let base = 2u64.saturating_pow(attempts.min(5));
    Duration::from_secs(base.min(60))
}

const RESET_AFTER_MS: u64 = 300_000;
