use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{LogLine, ProfileId, RequestId, RpcError, UnixMillis, SessionId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    Log(LogLine),
    Status(ServerStatus),
    Lifecycle(LifecycleEvent),
    RconOut { session: SessionId, text: String },
    RconErr { session: SessionId, text: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum Outbound {
    Response(Envelope<Response>),
    Event(Event),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub id: RequestId,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Request {
    Ping { client_version: String, protocol_version: u32 },

    Status {},

    Start {
        profile: ProfileId,
        #[serde(default)]
        env: BTreeMap<String, String>,
    },

    Stop {
        force: bool,
        grace_ms: Option<u64>,
    },

    Shutdown {},

    LogsTail { lines: usize },

    DaemonLogsTail { lines: usize },

    Subscribe {
        topics: Vec<Topic>,
        send_initial_status: bool,
    },

    Unsubscribe {},

    RconExec { command: String },
    RconOpen {},
    RconSend { session: SessionId, command: String },
    RconClose { session: SessionId },

    SaveDeployKey {
        hub_url: String,
        pack_id: String,
        channel: String,
        deploy_key: String,
        #[serde(default)]
        prefix: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Topic {
    Logs,
    Status,
    Lifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Response {
    Pong { daemon_version: String, protocol_version: u32 },

    Status { daemon: DaemonStatus, server: ServerStatus },

    Started { profile: ProfileId, pid: i32, started_at_ms: UnixMillis },
    Stopped { exit: Option<ExitInfo>, stopped_at_ms: UnixMillis },

    LogsTail { lines: Vec<LogLine>, truncated: bool },

    Subscribed { topics: Vec<Topic> },
    Unsubscribed {},

    ShutdownAck {},

    RconResult { text: String },
    RconOpened { session: SessionId, prompt: String },
    RconClosed { session: SessionId },

    DeployKeySaved {},

    Error(RpcError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub daemon_version: String,
    pub protocol_version: u32,
    pub pid: i32,
    pub uptime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", content = "data")]
pub enum ServerStatus {
    Idle {},

    Starting { profile: ProfileId, since_ms: UnixMillis },

    Running {
        profile: ProfileId,
        pid: i32,
        started_at_ms: UnixMillis,
        #[serde(default)]
        meta: BTreeMap<String, String>,
    },

    Stopping { profile: ProfileId, pid: i32, since_ms: UnixMillis },

    Exited { profile: ProfileId, exit: ExitInfo, at_ms: UnixMillis },
    Crashed { profile: ProfileId, exit: ExitInfo, at_ms: UnixMillis, last_logs: Vec<LogLine> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExitInfo {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    DaemonShuttingDown { at_ms: UnixMillis },
    ServerSpawned { pid: i32, at_ms: UnixMillis },
    ServerExited { exit: ExitInfo, at_ms: UnixMillis },
}
