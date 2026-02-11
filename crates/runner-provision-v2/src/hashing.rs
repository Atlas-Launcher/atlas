use crate::errors::ProvisionError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(crate) fn sha256_extracted_tree(
    root: &Path,
    exclude: &[PathBuf],
) -> Result<String, ProvisionError> {
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

#[cfg(test)]
mod tests {
    use super::{sha256_extracted_tree, verify_extracted_jdk_hash};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("atlas-runner-provision-{prefix}-{nanos}"))
    }

    #[test]
    fn tree_hash_changes_when_runtime_contents_change() {
        let dir = unique_temp_dir("tree-hash");
        std::fs::create_dir_all(dir.join("bin")).expect("create bin dir");
        std::fs::write(dir.join("bin").join("java"), b"java-a").expect("write java file");

        let first = sha256_extracted_tree(&dir, &[]).expect("hash first tree");
        std::fs::write(dir.join("bin").join("java"), b"java-b").expect("rewrite java file");
        let second = sha256_extracted_tree(&dir, &[]).expect("hash second tree");

        assert_ne!(first, second);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn verify_extracted_jdk_hash_succeeds_then_detects_tampering() {
        let dir = unique_temp_dir("verify-hash");
        std::fs::create_dir_all(dir.join("bin")).expect("create bin dir");
        std::fs::create_dir_all(dir.join("lib")).expect("create lib dir");

        let java_bin = dir.join("bin").join("java");
        let checksum_path = dir.join("java.hash");
        std::fs::write(&java_bin, b"java-binary").expect("write java binary");
        std::fs::write(dir.join("lib").join("rt.jar"), b"runtime-bytes").expect("write runtime");

        let expected = sha256_extracted_tree(&dir, std::slice::from_ref(&checksum_path))
            .expect("hash extracted tree");
        tokio::fs::write(&checksum_path, format!("{expected}\n"))
            .await
            .expect("write checksum");

        verify_extracted_jdk_hash(&dir, &java_bin)
            .await
            .expect("checksum should match");

        std::fs::write(&java_bin, b"tampered-java-binary").expect("tamper java binary");
        let err = verify_extracted_jdk_hash(&dir, &java_bin)
            .await
            .expect_err("checksum mismatch expected");
        assert!(err.to_string().contains("checksum mismatch"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
