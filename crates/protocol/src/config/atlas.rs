use serde::{Deserialize, Serialize};

use crate::{Loader, ProtocolError};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AtlasConfig {
    pub metadata: MetadataConfig,
    pub versions: VersionsConfig,
    pub cli: Option<CliConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetadataConfig {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VersionsConfig {
    pub mc: String,
    pub modloader: String,
    pub modloader_version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CliConfig {
    pub pack_id: Option<String>,
    pub hub_url: Option<String>,
    pub default_channel: Option<String>,
}

pub fn parse_config(contents: &str) -> Result<AtlasConfig, ProtocolError> {
    toml::from_str(contents).map_err(|_| ProtocolError::MissingField("atlas.toml"))
}

pub fn parse_loader(value: &str) -> Result<Loader, ProtocolError> {
    match value.to_lowercase().as_str() {
        "fabric" => Ok(Loader::Fabric),
        "forge" => Ok(Loader::Forge),
        "neo" | "neoforge" => Ok(Loader::Neo),
        _ => Err(ProtocolError::MissingField("versions.modloader")),
    }
}
