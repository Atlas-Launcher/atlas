use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use crate::errors::ProvisionError;

pub(crate) fn sha256_extracted_tree(root: &Path, exclude: &[PathBuf]) -> Result<String, ProvisionError> {
    // Collect all regular files, sorted by relative path.
    let mut files = Vec::<PathBuf>::new();

    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|e| ProvisionError::Invalid(format!("walk jdk failed: {e}")))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();

        if exclude.iter().any(|ex| ex == &path) {
            continue;
        }
        files.push(path);
    }

    files.sort_by(|a, b| {
        a.strip_prefix(root)
            .unwrap_or(a)
            .as_os_str()
            .cmp(b.strip_prefix(root).unwrap_or(b).as_os_str())
    });

    let mut h = Sha256::new();

    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(&path);
        // bind path into the hash (prevents reordering/collision tricks)
        h.update(rel.to_string_lossy().as_bytes());
        h.update([0u8]); // separator

        let bytes = std::fs::read(&path)?;
        h.update(&bytes);
        h.update([0u8]); // separator
    }

    Ok(hex::encode(h.finalize()))
}

pub async fn verify_extracted_jdk_hash(
    install_dir: &Path,
    _java_bin: &Path,
) -> Result<(), ProvisionError> {
    let checksum_path = install_dir.join("java.hash");
    if !checksum_path.exists() {
        return Err(ProvisionError::Invalid(format!(
            "missing java checksum file: {}",
            checksum_path.display()
        )));
    }

    let expected = tokio::fs::read_to_string(&checksum_path).await?;
    let expected = expected.trim().to_ascii_lowercase();

    let install_dir_clone = install_dir.to_path_buf();
    let checksum_path_clone = checksum_path.clone();

    let actual = tokio::task::spawn_blocking(move || {
        sha256_extracted_tree(&install_dir_clone, &[checksum_path_clone])
    })
        .await
        .map_err(|e| ProvisionError::Invalid(format!("java hash task failed: {e}")))??;

    if actual != expected {
        return Err(ProvisionError::Invalid(format!(
            "java install checksum mismatch: expected {expected}, got {actual}"
        )));
    }

    Ok(())
}

