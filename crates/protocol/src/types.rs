use crate::platform::PlatformFilter;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMetadata {
    pub pack_id: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: Loader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub dependencies: Vec<Dependency>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub url: String,
    pub hash: Hash,
    pub platform: PlatformFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    pub hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ::prost::Enumeration)]
#[repr(i32)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha1 = 0,
    Sha256 = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ::prost::Enumeration)]
#[repr(i32)]
#[serde(rename_all = "snake_case")]
pub enum Loader {
    Fabric = 0,
    Forge = 1,
    Neo = 2,
}

pub type ByteMap = BTreeMap<String, Vec<u8>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackBlob {
    pub metadata: PackMetadata,
    pub manifest: Manifest,
    pub files: ByteMap,
}
