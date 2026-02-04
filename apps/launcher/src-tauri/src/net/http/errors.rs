use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Request failed ({status}): {body}")]
    Status { status: StatusCode, body: String },
    #[error("Failed to parse JSON: {source}. Body: {body}")]
    Parse {
        source: serde_json::Error,
        body: String,
    },
    #[error("Failed to parse response: {message}. Body: {body}")]
    ParseMessage { message: String, body: String },
}
