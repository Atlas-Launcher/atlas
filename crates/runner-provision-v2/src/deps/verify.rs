use protocol::{Dependency, HashAlgorithm};
use crate::errors::ProvisionError;

pub fn verify_dependency_bytes(dep: &Dependency, bytes: &[u8]) -> Result<(), ProvisionError> {
    let h = dep.hash.decode_hex_bytes().or_else(|_| {
        Err(ProvisionError::Invalid(format!(
            "missing hash for dependency at URL {}",
            dep.url
        )))
    })?;
    let expected = dep.hash.hex.clone();

    let actual = match HashAlgorithm::try_from(dep.hash.algorithm).unwrap_or(HashAlgorithm::Sha256) {
        HashAlgorithm::Sha1 => {
            use sha1::Digest;
            let mut hasher = sha1::Sha1::new();
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        }
        HashAlgorithm::Sha256 => {
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        }
        HashAlgorithm::Sha512 => {
            use sha2::Digest;
            let mut hasher = sha2::Sha512::new();
            hasher.update(bytes);
            hex::encode(hasher.finalize())
        }
    };

    if actual != expected {
        return Err(ProvisionError::Integrity {
            url: dep.url.clone(),
            expected,
            actual,
        });
    }
    Ok(())
}
