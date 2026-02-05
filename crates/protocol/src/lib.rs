mod codec;
pub mod config;
mod error;
pub mod pack;
mod platform;
mod types;
mod wire;

pub use crate::codec::{decode_blob, encode_blob, encode_blob_default, DEFAULT_ZSTD_LEVEL};
pub use crate::config::*;
pub use crate::error::ProtocolError;
pub use crate::pack::*;
pub use crate::platform::{Platform, PlatformFilter};
pub use crate::types::{
    ByteMap, Dependency, Hash, HashAlgorithm, Loader, Manifest, PackBlob, PackMetadata,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_blob() {
        let mut files = ByteMap::new();
        files.insert(
            "config/server.properties".to_string(),
            b"motd=Atlas".to_vec(),
        );

        let blob = PackBlob {
            metadata: PackMetadata {
                pack_id: "atlas".to_string(),
                version: "1.2.3".to_string(),
                minecraft_version: "1.20.1".to_string(),
                loader: Loader::Fabric,
            },
            manifest: Manifest {
                dependencies: vec![Dependency {
                    url: "https://example.com/mod.jar".to_string(),
                    hash: Hash {
                        algorithm: HashAlgorithm::Sha256,
                        hex: "deadbeef".to_string(),
                    },
                    platform: PlatformFilter::default(),
                }],
            },
            files,
        };

        let encoded = encode_blob_default(&blob).expect("encode failed");
        let decoded = decode_blob(&encoded).expect("decode failed");

        assert_eq!(blob, decoded);
    }

    #[test]
    fn platform_filter_allows() {
        let filter = PlatformFilter {
            include: vec![Platform::Linux],
            exclude: vec![Platform::Macos],
        };

        assert!(filter.allows(Platform::Linux));
        assert!(!filter.allows(Platform::Windows));
        assert!(!filter.allows(Platform::Macos));
    }
}
