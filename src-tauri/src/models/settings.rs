use serde::{Deserialize, Serialize};

use crate::paths;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub ms_client_id: Option<String>,
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
            memory_mb: default_memory_mb(),
        };
        Self {
            ms_client_id: None,
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
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
}

pub fn default_memory_mb() -> u32 {
    4096
}
