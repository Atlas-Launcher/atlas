use crate::error::ProtocolError;
use crate::platform::Platform;
use crate::types::{HashAlgorithm, Loader};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/atlas.protocol.rs"));
}

pub use proto::{Dependency, Hash, Manifest, PackBlob, PackMetadata, PlatformFilter};

impl From<&crate::types::PackMetadata> for PackMetadata {
    fn from(value: &crate::types::PackMetadata) -> Self {
        Self {
            pack_id: value.pack_id.clone(),
            version: value.version.clone(),
            minecraft_version: value.minecraft_version.clone(),
            loader: value.loader as i32,
        }
    }
}

impl TryFrom<PackMetadata> for crate::types::PackMetadata {
    type Error = ProtocolError;

    fn try_from(value: PackMetadata) -> Result<Self, Self::Error> {
        Ok(Self {
            pack_id: value.pack_id,
            version: value.version,
            minecraft_version: value.minecraft_version,
            loader: decode_loader(value.loader)?,
        })
    }
}

impl From<&crate::types::Manifest> for Manifest {
    fn from(value: &crate::types::Manifest) -> Self {
        Self {
            dependencies: value.dependencies.iter().map(Dependency::from).collect(),
        }
    }
}

impl TryFrom<Manifest> for crate::types::Manifest {
    type Error = ProtocolError;

    fn try_from(value: Manifest) -> Result<Self, Self::Error> {
        let dependencies = value
            .dependencies
            .into_iter()
            .map(crate::types::Dependency::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { dependencies })
    }
}

impl From<&crate::types::Dependency> for Dependency {
    fn from(value: &crate::types::Dependency) -> Self {
        Self {
            url: value.url.clone(),
            hash: Some(Hash::from(&value.hash)),
            platform: Some(PlatformFilter::from(&value.platform)),
        }
    }
}

impl TryFrom<Dependency> for crate::types::Dependency {
    type Error = ProtocolError;

    fn try_from(value: Dependency) -> Result<Self, Self::Error> {
        let hash = value
            .hash
            .ok_or(ProtocolError::MissingField("dependency.hash"))?
            .try_into()?;
        let platform = value
            .platform
            .map(crate::platform::PlatformFilter::try_from)
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            url: value.url,
            hash,
            platform,
        })
    }
}

impl From<&crate::types::Hash> for Hash {
    fn from(value: &crate::types::Hash) -> Self {
        Self {
            algorithm: value.algorithm as i32,
            hex: value.hex.clone(),
        }
    }
}

impl TryFrom<Hash> for crate::types::Hash {
    type Error = ProtocolError;

    fn try_from(value: Hash) -> Result<Self, Self::Error> {
        Ok(Self {
            algorithm: decode_hash_algorithm(value.algorithm)?,
            hex: value.hex,
        })
    }
}

impl From<&crate::platform::PlatformFilter> for PlatformFilter {
    fn from(value: &crate::platform::PlatformFilter) -> Self {
        Self {
            include: value
                .include
                .iter()
                .map(|platform| *platform as i32)
                .collect(),
            exclude: value
                .exclude
                .iter()
                .map(|platform| *platform as i32)
                .collect(),
        }
    }
}

impl TryFrom<PlatformFilter> for crate::platform::PlatformFilter {
    type Error = ProtocolError;

    fn try_from(value: PlatformFilter) -> Result<Self, Self::Error> {
        Ok(Self {
            include: decode_platforms("platform_filter.include", value.include)?,
            exclude: decode_platforms("platform_filter.exclude", value.exclude)?,
        })
    }
}

impl TryFrom<&crate::types::PackBlob> for PackBlob {
    type Error = ProtocolError;

    fn try_from(value: &crate::types::PackBlob) -> Result<Self, Self::Error> {
        Ok(Self {
            metadata: Some(PackMetadata::from(&value.metadata)),
            manifest: Some(Manifest::from(&value.manifest)),
            files: value.files.clone(),
        })
    }
}

impl TryFrom<PackBlob> for crate::types::PackBlob {
    type Error = ProtocolError;

    fn try_from(value: PackBlob) -> Result<Self, Self::Error> {
        let metadata = value
            .metadata
            .ok_or(ProtocolError::MissingField("pack_blob.metadata"))?
            .try_into()?;
        let manifest = value
            .manifest
            .ok_or(ProtocolError::MissingField("pack_blob.manifest"))?
            .try_into()?;

        Ok(Self {
            metadata,
            manifest,
            files: value.files,
        })
    }
}

fn decode_loader(value: i32) -> Result<Loader, ProtocolError> {
    Loader::try_from(value).map_err(|_| ProtocolError::InvalidEnum {
        field: "loader",
        value,
    })
}

fn decode_hash_algorithm(value: i32) -> Result<HashAlgorithm, ProtocolError> {
    HashAlgorithm::try_from(value).map_err(|_| ProtocolError::InvalidEnum {
        field: "hash.algorithm",
        value,
    })
}

fn decode_platforms(field: &'static str, values: Vec<i32>) -> Result<Vec<Platform>, ProtocolError> {
    values
        .into_iter()
        .map(|value| {
            Platform::try_from(value).map_err(|_| ProtocolError::InvalidEnum { field, value })
        })
        .collect()
}
