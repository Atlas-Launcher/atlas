use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProvisionError {
    #[error("invalid pack: {0}")]
    Invalid(String),

    #[error("integrity check failed for {url}: expected {expected}, got {actual}")]
    Integrity { url: String, expected: String, actual: String },

    #[error("decode error: {0}")]
    Decode(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
    
    #[error(transparent)]
    MissingDependency(#[from] Box<dyn std::error::Error + Send + Sync>),
}