use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("zstd error: {0}")]
    Zstd(#[from] std::io::Error),
}
