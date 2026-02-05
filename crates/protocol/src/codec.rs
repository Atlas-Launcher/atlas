use crate::error::ProtocolError;
use crate::types::PackBlob;
use std::io::Cursor;

pub const DEFAULT_ZSTD_LEVEL: i32 = 19;

pub fn encode_blob(blob: &PackBlob, zstd_level: i32) -> Result<Vec<u8>, ProtocolError> {
    let encoded = bincode::serialize(blob)?;
    let compressed = zstd::stream::encode_all(Cursor::new(encoded), zstd_level)?;
    Ok(compressed)
}

pub fn encode_blob_default(blob: &PackBlob) -> Result<Vec<u8>, ProtocolError> {
    encode_blob(blob, DEFAULT_ZSTD_LEVEL)
}

pub fn decode_blob(bytes: &[u8]) -> Result<PackBlob, ProtocolError> {
    let decompressed = zstd::stream::decode_all(Cursor::new(bytes))?;
    let blob = bincode::deserialize(&decompressed)?;
    Ok(blob)
}
