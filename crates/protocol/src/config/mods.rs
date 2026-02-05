use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModEntry {
    pub source: String,
    pub project_id: String,
    pub version: String,
    pub file_id: Option<String>,
    pub download_url: Option<String>,
    pub hashes: Option<ModHashes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModHashes {
    pub sha1: Option<String>,
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
