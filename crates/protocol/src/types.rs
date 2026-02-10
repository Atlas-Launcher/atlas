use crate::platform::PlatformFilter;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMetadata {
    pub pack_id: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: Loader,
    pub loader_version: String,
    pub name: String,
    pub description: String,
}

impl AsRef<PackMetadata> for PackMetadata {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub dependencies: Vec<Dependency>,
}

impl AsRef<Manifest> for Manifest {
    fn as_ref(&self) -> &Self {
        self
    }
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
    pub kind: DependencyKind,
    pub side: DependencySide,
    pub pointer_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    pub hex: String,
}

impl Hash {
    pub fn decode_hex_bytes(&self) -> Result<Vec<u8>, &'static str> {
        if self.hex.len() % 2 != 0 {
            return Err("invalid hex length");
        }
        hex::decode(&self.hex).map_err(|_| "invalid hex string")
    }
}
impl AsRef<Hash> for Hash {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ::prost::Enumeration)]
#[repr(i32)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha1 = 0,
    Sha256 = 1,
    Sha512 = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ::prost::Enumeration)]
#[repr(i32)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    Mod = 0,
    Resource = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ::prost::Enumeration)]
#[repr(i32)]
#[serde(rename_all = "snake_case")]
pub enum DependencySide {
    Both = 0,
    Client = 1,
    Server = 2,
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
