use thiserror::Error;

use crate::net::http::HttpError;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

impl From<String> for AuthError {
    fn from(value: String) -> Self {
        AuthError::Message(value)
    }
}
