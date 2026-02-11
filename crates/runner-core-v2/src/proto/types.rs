use serde::{Deserialize, Serialize};

pub type RequestId = u64;
pub type UnixMillis = u64;

pub type ProfileId = String;

pub type SessionId = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub at_ms: UnixMillis,
    pub stream: LogStream,
    pub line: String,
}
