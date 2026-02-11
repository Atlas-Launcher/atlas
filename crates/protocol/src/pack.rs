use crate::{
    Dependency, DependencyKind, DependencySide, Hash, HashAlgorithm, Manifest, PackBlob,
    PackMetadata, Platform, PlatformFilter, ProtocolError, config::atlas, config::mods,
    encode_blob,
};
use std::collections::BTreeMap;

pub struct BuildInput {
    pub pack_id: String,
    pub config: atlas::AtlasConfig,
    pub files: BTreeMap<String, Vec<u8>>,
    pub version_override: Option<String>,
}

pub struct BuildOutput {
    pub bytes: Vec<u8>,
    pub metadata: PackMetadata,
}

pub fn build_pack_bytes(input: BuildInput, zstd_level: i32) -> Result<BuildOutput, ProtocolError> {
    let loader = atlas::parse_loader(&input.config.versions.modloader)?;

    let version = input
        .version_override
        .or(input.config.metadata.version)
        .ok_or(ProtocolError::MissingField("metadata.version"))?;

    let metadata = PackMetadata {
        pack_id: input.pack_id,
        version,
        minecraft_version: input.config.versions.mc,
        loader,
        loader_version: input.config.versions.modloader_version,
        name: input.config.metadata.name,
        description: input.config.metadata.description.unwrap_or_default(),
    };

    let manifest = build_manifest(&input.files)?;

    let blob = PackBlob {
        metadata: metadata.clone(),
        manifest,
        files: input.files,
    };

    let encoded = encode_blob(&blob, zstd_level)?;
    Ok(BuildOutput {
        bytes: encoded,
        metadata,
    })
}

fn build_manifest(files: &BTreeMap<String, Vec<u8>>) -> Result<Manifest, ProtocolError> {
    let mut dependencies = Vec::new();

    for (path, bytes) in files {
        let Some(kind) = dependency_kind_from_path(path) else {
            continue;
        };

        let contents =
            std::str::from_utf8(bytes).map_err(|_| ProtocolError::MissingField("pointer.toml"))?;
        let entry = mods::parse_mod_toml(contents)
            .map_err(|_| ProtocolError::MissingField("pointer.toml"))?;

        let url = entry
            .download
            .url
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or(ProtocolError::MissingField("download.url"))?;

        let hash = select_hash(entry.download.hashes.as_ref())
            .ok_or(ProtocolError::MissingField("download.hashes"))?;

        let (side, platform) = map_side_and_platform(&entry.metadata);

        dependencies.push(Dependency {
            url,
            hash,
            platform,
            kind,
            side,
            pointer_path: path.clone(),
        });
    }

    Ok(Manifest { dependencies })
}

fn dependency_kind_from_path(path: &str) -> Option<DependencyKind> {
    if path.ends_with(".mod.toml") {
        Some(DependencyKind::Mod)
    } else if path.ends_with(".res.toml") {
        Some(DependencyKind::Resource)
    } else {
        None
    }
}

fn select_hash(hashes: Option<&mods::ModHashes>) -> Option<Hash> {
    let hashes = hashes?;

    if let Some(hex) = hashes.sha512.as_ref() {
        return Some(Hash {
            algorithm: HashAlgorithm::Sha512,
            hex: hex.clone(),
        });
    }

    if let Some(hex) = hashes.sha256.as_ref() {
        return Some(Hash {
            algorithm: HashAlgorithm::Sha256,
            hex: hex.clone(),
        });
    }

    hashes.sha1.as_ref().map(|hex| Hash {
        algorithm: HashAlgorithm::Sha1,
        hex: hex.clone(),
    })
}

fn map_side_and_platform(metadata: &mods::ModMetadata) -> (DependencySide, PlatformFilter) {
    let side = match metadata.side {
        mods::ModSide::Client => DependencySide::Client,
        mods::ModSide::Server => DependencySide::Server,
        mods::ModSide::Both => DependencySide::Both,
    };
    let mut exclude = Vec::new();

    for os in &metadata.disabled_client_oses {
        let platform = match os {
            mods::ClientOs::Windows => Platform::Windows,
            mods::ClientOs::Linux => Platform::Linux,
            mods::ClientOs::Macos => Platform::Macos,
        };
        exclude.push(platform);
    }

    (
        side,
        PlatformFilter {
            include: Vec::new(),
            exclude,
        },
    )
}
