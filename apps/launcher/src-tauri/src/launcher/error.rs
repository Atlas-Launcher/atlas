use thiserror::Error;

use crate::net::http::HttpError;

#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

impl From<String> for LauncherError {
    fn from(value: String) -> Self {
        LauncherError::Message(value)
    }
}
