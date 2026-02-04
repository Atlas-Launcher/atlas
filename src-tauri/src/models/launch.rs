use serde::{Deserialize, Serialize};

use super::settings::{default_memory_mb, ModLoaderConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    #[serde(default)]
    pub game_dir: String,
    #[serde(default)]
    pub java_path: String,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub loader: ModLoaderConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LaunchEvent {
    pub phase: String,
    pub message: String,
    #[serde(default)]
    pub current: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub percent: Option<u64>,
}
