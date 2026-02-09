use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: ErrorCode,
    pub message: String,

    #[serde(default)]
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    BadRequest,
    UnsupportedProtocol,

    DaemonBusy,
    ServerAlreadyRunning,
    ServerNotRunning,

    UnknownProfile,
    InvalidConfig,
    IoError,

    Internal,
}
