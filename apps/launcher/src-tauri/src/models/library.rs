use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifestSummary {
    pub latest_release: String,
    pub versions: Vec<VersionSummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FabricLoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub file_name: String,
    pub display_name: String,
    pub enabled: bool,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AtlasRemotePack {
    pub pack_id: String,
    pub pack_name: String,
    pub pack_slug: String,
    pub access_level: String,
    pub channel: String,
    #[serde(default)]
    pub build_id: Option<String>,
    #[serde(default)]
    pub build_version: Option<String>,
    #[serde(default)]
    pub artifact_key: Option<String>,
}
