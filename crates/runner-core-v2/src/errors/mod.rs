use std::path::PathBuf;

use thiserror::Error;

use crate::proto::{ErrorCode, RpcError};

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unsupported protocol version: client={client} daemon={daemon}")]
    UnsupportedProtocol { client: u32, daemon: u32 },

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("unknown profile: {0}")]
    UnknownProfile(String),

    #[error("I/O error while {context}: {source}")]
    Io {
        context: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("path error: {0}")]
    Path(String),

    #[error("internal error: {0}")]
    Internal(String),
}

// Convenience constructors (optional)
impl CoreError {
    pub fn io(context: &'static str, source: std::io::Error) -> Self {
        Self::Io { context, source }
    }

    pub fn path(p: impl Into<PathBuf>) -> Self {
        Self::Path(format!("invalid path: {}", p.into().display()))
    }
}

/// Map internal errors -> stable wire errors.
/// Keep this mapping conservative and stable.
impl From<CoreError> for RpcError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::UnsupportedProtocol { client, daemon } => RpcError {
                code: ErrorCode::UnsupportedProtocol,
                message: e.to_string(),
                details: [
                    ("client_protocol".into(), client.to_string()),
                    ("daemon_protocol".into(), daemon.to_string()),
                ]
                .into_iter()
                .collect(),
            },
            CoreError::InvalidConfig(_) => RpcError {
                code: ErrorCode::InvalidConfig,
                message: e.to_string(),
                details: Default::default(),
            },
            CoreError::UnknownProfile(_) => RpcError {
                code: ErrorCode::UnknownProfile,
                message: e.to_string(),
                details: Default::default(),
            },
            CoreError::Io { context, .. } => RpcError {
                code: ErrorCode::IoError,
                message: e.to_string(),
                details: [("context".into(), context.into())].into_iter().collect(),
            },
            CoreError::Path(_) => RpcError {
                code: ErrorCode::InvalidConfig,
                message: e.to_string(),
                details: Default::default(),
            },
            CoreError::Internal(_) => RpcError {
                code: ErrorCode::Internal,
                message: e.to_string(),
                details: Default::default(),
            },
        }
    }
}
