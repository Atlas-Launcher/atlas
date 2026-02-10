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

            {
                let mut guard = state.lock().await;
                if let Some(child) = guard.child.as_mut() {
                    if let Ok(Some(status)) = child.try_wait() {
                        let uptime_ms = guard
                            .last_start_ms
                            .map(|start| now_millis().saturating_sub(start))
                            .unwrap_or(0);
                        let exit_code = status.code();
                        let exit = ExitInfo { code: exit_code, signal: None };
                        let profile = guard.profile.clone().unwrap_or_else(|| "default".into());
                        guard.child = None;
                        guard.status = ServerStatus::Exited {
                            profile: profile.clone(),
                            exit,
                            at_ms: now_millis(),
                        };
                        let logs = guard.logs.clone();
                        logs.push_daemon(format!(
                            "server crashed: profile={} exit_code={:?}",
                            profile,
                            exit_code
                        ));
                        guard.restart_disabled = true;
                        continue;
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }

        }
    });
}
