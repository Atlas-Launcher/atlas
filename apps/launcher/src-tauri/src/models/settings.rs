use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub ms_client_id: Option<String>,
    #[serde(default)]
    pub atlas_hub_url: Option<String>,
    #[serde(default = "default_memory_mb")]
    pub default_java_memory_mb: u32,
    #[serde(default)]
    pub default_jvm_args: Option<String>,
    #[serde(default)]
    pub instances: Vec<InstanceConfig>,
    #[serde(default)]
    pub selected_instance_id: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let game_dir = paths::default_game_dir().to_string_lossy().to_string();
        let instance = InstanceConfig {
            id: "default".to_string(),
            name: "Default".to_string(),
            game_dir,
            version: None,
            loader: ModLoaderConfig::default(),
            java_path: String::new(),
            memory_mb: None,
            jvm_args: None,
            source: InstanceSource::default(),
            atlas_pack: None,
        };
        Self {
            ms_client_id: None,
            atlas_hub_url: None,
            default_java_memory_mb: default_memory_mb(),
            default_jvm_args: None,
            instances: vec![instance],
            selected_instance_id: Some("default".to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModLoaderConfig {
    #[serde(default)]
    pub kind: ModLoaderKind,
    #[serde(default)]
    pub loader_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ModLoaderKind {
    Vanilla,
    Fabric,
    NeoForge,
}

impl Default for ModLoaderKind {
    fn default() -> Self {
        ModLoaderKind::Vanilla
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub game_dir: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub loader: ModLoaderConfig,
    #[serde(default)]
    pub java_path: String,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub jvm_args: Option<String>,
    #[serde(default)]
    pub source: InstanceSource,
    #[serde(default)]
    pub atlas_pack: Option<AtlasPackLink>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum InstanceSource {
    Local,
    Atlas,
}

impl Default for InstanceSource {
    fn default() -> Self {
        InstanceSource::Local
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AtlasPackLink {
    pub pack_id: String,
    pub pack_slug: String,
    pub channel: String,
    #[serde(default)]
    pub build_id: Option<String>,
    #[serde(default)]
    pub build_version: Option<String>,
    #[serde(default)]
    pub artifact_key: Option<String>,
}

pub fn default_memory_mb() -> u32 {
    4096
}
