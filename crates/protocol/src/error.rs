use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),
    #[error("protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),
    #[error("zstd error: {0}")]
    Zstd(#[from] std::io::Error),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid enum value for {field}: {value}")]
    InvalidEnum { field: &'static str, value: i32 },
}
