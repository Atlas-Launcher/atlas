use thiserror::Error;

use crate::net::http::HttpError;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<String> for LibraryError {
    fn from(value: String) -> Self {
        LibraryError::Message(value)
    }
}
