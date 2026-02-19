use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModEntry {
    #[serde(default)]
    pub metadata: ModMetadata,
    #[serde(default, skip_serializing_if = "ModCompat::is_empty")]
    pub compat: ModCompat,
    pub download: ModDownload,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ModCompat {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub minecraft: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loader_versions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<ModCompatDependency>,
}

impl ModCompat {
    pub fn is_empty(&self) -> bool {
        self.minecraft.is_empty()
            && self.loaders.is_empty()
            && self.loader_versions.is_empty()
            && self.requires.is_empty()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModCompatDependency {
    pub source: String,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub side: ModSide,
    #[serde(default)]
    pub project_url: Option<String>,
    #[serde(default)]
    pub disabled_client_oses: Vec<ClientOs>,
}

impl Default for ModMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            side: ModSide::Both,
            project_url: None,
            disabled_client_oses: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModSide {
    Client,
    Server,
    Both,
}

impl Default for ModSide {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClientOs {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModDownload {
    pub source: String,
    pub project_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashes: Option<ModHashes>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha512: Option<String>,
}

impl ModEntry {
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string(self)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyModEntry {
    #[serde(default)]
    metadata: Option<ModMetadata>,
    source: String,
    project_id: String,
    version: String,
    #[serde(default)]
    file_id: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    hashes: Option<ModHashes>,
}

pub fn parse_mod_toml(contents: &str) -> Result<ModEntry, toml::de::Error> {
    match toml::from_str::<ModEntry>(contents) {
        Ok(parsed) => Ok(parsed),
        Err(_) => {
            let legacy = toml::from_str::<LegacyModEntry>(contents)?;
            let mut metadata = legacy.metadata.unwrap_or_default();
            if metadata.name.trim().is_empty() {
                metadata.name = legacy.project_id.clone();
            }
            Ok(ModEntry {
                metadata,
                compat: ModCompat::default(),
                download: ModDownload {
                    source: legacy.source,
                    project_id: legacy.project_id,
                    version: legacy.version,
                    file_id: legacy.file_id,
                    url: legacy.download_url,
                    hashes: legacy.hashes,
                },
            })
        }
    }
}
