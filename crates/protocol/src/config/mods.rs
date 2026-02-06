use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ModMetadata>,
    pub source: String,
    pub project_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashes: Option<ModHashes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModMetadata {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
}

impl ModEntry {
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}

pub fn parse_mod_toml(contents: &str) -> Result<ModEntry, toml::de::Error> {
    toml::from_str(contents)
}
