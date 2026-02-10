use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use std::sync::atomic::AtomicBool;
use tokio::task::JoinHandle;

use runner_core_v2::proto::{ProfileId, ServerStatus};
use runner_provision_v2::LaunchPlan;

use super::logs::LogStore;

pub type SharedState = Arc<Mutex<ServerState>>;

pub struct ServerState {
    pub(crate) status: ServerStatus,
    pub(crate) child: Option<Child>,
    pub(crate) profile: Option<ProfileId>,
    pub(crate) server_root: Option<PathBuf>,
    pub(crate) launch_plan: Option<LaunchPlan>,
    pub(crate) restart_attempts: u32,
    pub(crate) restart_disabled: bool,
    pub(crate) watchers_started: bool,
    pub(crate) monitor_started: bool,
    pub(crate) last_start_ms: Option<u64>,
    pub(crate) logs: LogStore,
    pub(crate) pack_etag: Option<String>,
    pub(crate) whitelist_etag: Option<String>,
    pub(crate) current_pack_build_id: Option<String>,
    pub(crate) watcher_stop: Option<Arc<AtomicBool>>,
    // Optional JoinHandle for the watcher worker thread so the daemon can wait for it to exit
    pub(crate) watcher_handle: Option<JoinHandle<()>>,
    // Flag set by watcher worker when it has fully exited
    pub(crate) watcher_done: Option<Arc<AtomicBool>>,
    // Serialize start/stop/update operations so only one lifecycle operation runs at once
    pub(crate) lifecycle_lock: Arc<tokio::sync::Mutex<()>>,
}

impl ServerState {
    pub fn new(logs: LogStore) -> Self {
        Self {
            status: ServerStatus::Idle {},
            child: None,
            profile: None,
            server_root: None,
            launch_plan: None,
            restart_attempts: 0,
            restart_disabled: false,
            watchers_started: false,
            monitor_started: false,
            last_start_ms: None,
            logs,
            pack_etag: None,
            whitelist_etag: None,
            current_pack_build_id: None,
            watcher_stop: None,
            watcher_handle: None,
            watcher_done: None,
            lifecycle_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, ServerStatus::Running { .. } | ServerStatus::Starting { .. })
    }
}
