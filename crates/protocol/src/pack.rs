use crate::{Manifest, PackBlob, PackMetadata, ProtocolError, config::atlas, encode_blob};
use std::collections::BTreeMap;

pub struct BuildInput {
    pub pack_id: String,
    pub config: atlas::AtlasConfig,
    pub files: BTreeMap<String, Vec<u8>>,
    pub version_override: Option<String>,
}

pub struct BuildOutput {
    pub bytes: Vec<u8>,
    pub metadata: PackMetadata,
}

pub fn build_pack_bytes(input: BuildInput, zstd_level: i32) -> Result<BuildOutput, ProtocolError> {
    let loader = atlas::parse_loader(&input.config.versions.modloader)?;

    let version = input
        .version_override
        .or(input.config.metadata.version)
        .ok_or(ProtocolError::MissingField("metadata.version"))?;

    let metadata = PackMetadata {
        pack_id: input.pack_id,
        version,
        minecraft_version: input.config.versions.mc,
        loader,
    };

    let blob = PackBlob {
        metadata: metadata.clone(),
        manifest: Manifest::default(),
        files: input.files,
    };

    let encoded = encode_blob(&blob, zstd_level)?;
    Ok(BuildOutput {
        bytes: encoded,
        metadata,
    })
}
