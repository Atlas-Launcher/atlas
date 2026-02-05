use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Windows,
    Linux,
    Macos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformFilter {
    pub include: Vec<Platform>,
    pub exclude: Vec<Platform>,
}

impl Default for PlatformFilter {
    fn default() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

impl PlatformFilter {
    pub fn allows(&self, platform: Platform) -> bool {
        if !self.include.is_empty() && !self.include.contains(&platform) {
            return false;
        }
        if self.exclude.contains(&platform) {
            return false;
        }
        true
    }
}
