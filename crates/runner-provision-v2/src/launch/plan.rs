use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchPlan {
    pub cwd_rel: PathBuf,   // usually "."
    pub argv: Vec<String>,  // e.g. ["java","-Xmx8G","-jar","server.jar","nogui"]
}
