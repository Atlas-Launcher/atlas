use std::fs;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use log::info;
use crate::errors::ProvisionError;
use crate::hashing::{sha256_extracted_tree, verify_extracted_jdk_hash};
const JAVA_DIR_NAME: &str = "java";

pub async fn ensure_java_for_minecraft(
    server_root: &Path,
    mc_version: &str,
    override_major: Option<u32>,
) -> Result<PathBuf, ProvisionError> {
    let minimum = java_version_for_minecraft(mc_version);
    let major = match override_major {
        Some(value) if value >= minimum => value,
        Some(_) => minimum,
        None => minimum,
    };

    let os = std::env::consts::OS;
    if os != "linux" && os != "macos" {
        return Err(ProvisionError::Invalid(
            "java runtime install is only supported on linux or macos runner hosts".to_string(),
        ));
    }

    let install_root = server_root.join(".runner").join(JAVA_DIR_NAME);
    let install_dir = install_root.join(format!("jdk-{major}"));
    let java_bin = java_bin_path(&install_dir);

    if java_bin.exists() {
        if let Err(e) = verify_extracted_jdk_hash(&install_dir, &java_bin).await {
            // corrupted/modified install: remove and reinstall
            info!("java runtime at {} failed verification: {e}, reinstalling", install_dir.display());
            let _ = tokio::fs::remove_dir_all(&install_dir).await;
            // fall through to reinstall
        } else {
            return Ok(java_bin);
        }
    }

    tokio::fs::create_dir_all(&install_dir).await?;

    let archive_path = install_dir.join("download.tar.gz");
    download_jdk_archive(major, &archive_path).await?;

    let install_dir_clone = install_dir.clone();
    let archive_clone = archive_path.clone();
    tokio::task::spawn_blocking(move || extract_jdk_tar(&archive_clone, &install_dir_clone))
        .await
        .map_err(|err| {
            ProvisionError::Invalid(format!("java install task failed: {err}"))
        })??;

    let _ = tokio::fs::remove_file(&archive_path).await;

    if !java_bin.exists() {
        return Err(ProvisionError::Invalid(format!(
            "java install failed: {} not found",
            java_bin.display()
        )));
    }

    let checksum_path = install_dir.join("java.hash");

    // Compute & write a checksum of the extracted JDK tree.
    // Exclude the checksum file itself (and any temp files if you want).
    let install_dir_clone = install_dir.clone();
    let checksum_path_clone = checksum_path.clone();

    let tree_hash = tokio::task::spawn_blocking(move || {
        sha256_extracted_tree(&install_dir_clone, &[checksum_path_clone])
    })
        .await
        .map_err(|e| ProvisionError::Invalid(format!("java hash task failed: {e}")))??;

    // Atomic write
    let tmp_hash = install_dir.join("java.hash.tmp");
    tokio::fs::write(&tmp_hash, format!("{tree_hash}\n")).await?;
    tokio::fs::rename(&tmp_hash, &checksum_path).await?;

    Ok(java_bin)
}

fn java_version_for_minecraft(version: &str) -> u32 {
    let (major, minor) = parse_minecraft_version(version);
    if major > 1 || minor >= 20 {
        if minor >= 20 && version_at_least(version, (1, 20, 5)) {
            return 21;
        }
        return 17;
    }
    if minor >= 18 {
        return 17;
    }
    8
}

fn parse_minecraft_version(version: &str) -> (u32, u32) {
    let mut parts = version
        .split(|c| c == '.' || c == '-')
        .filter_map(|value| value.parse::<u32>().ok());
    let major = parts.next().unwrap_or(1);
    let minor = parts.next().unwrap_or(0);
    (major, minor)
}

fn version_at_least(version: &str, target: (u32, u32, u32)) -> bool {
    let mut parts = version
        .split(|c| c == '.' || c == '-')
        .filter_map(|value| value.parse::<u32>().ok());
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);

    (major, minor, patch) >= target
}

#[derive(Debug, Deserialize)]
struct Asset {
    binaries: Vec<Binary>,
}

#[derive(Debug, Deserialize)]
struct Binary {
    architecture: String,
    os: String,
    image_type: String,
    jvm_impl: String,
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    link: String,
    checksum: String, // sha256 hex
    // checksum_link: Option<String>,
}
pub async fn download_jdk_archive(major: u32, dest: &Path) -> Result<(), ProvisionError> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        other => {
            return Err(ProvisionError::Invalid(format!(
                "unsupported os: {other}"
            )))
        }
    };

    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => {
            return Err(ProvisionError::Invalid(format!(
                "unsupported architecture: {other}"
            )))
        }
    };

    let client = Client::new();

    // 1) Fetch metadata so we can get the sha256 for the exact artifact.
    //
    // Using feature_releases assets endpoint; filter via query params to reduce ambiguity.
    // vendor=adoptium, image_type=jdk, jvm_impl=hotspot, heap_size=normal
    let meta_url = format!(
        "https://api.adoptium.net/v3/assets/feature_releases/{major}/ga\
?architecture={arch}&heap_size=normal&image_type=jdk&jvm_impl=hotspot&os={os}&vendor=adoptium"
    );

    let assets: Vec<Asset> = client
        .get(&meta_url)
        .send()
        .await
        .map_err(|e| ProvisionError::Invalid(format!("adoptium metadata fetch failed: {e}")))?
        .error_for_status()
        .map_err(|e| ProvisionError::Invalid(format!("adoptium metadata fetch failed: {e}")))?
        .json()
        .await
        .map_err(|e| ProvisionError::Invalid(format!("adoptium metadata parse failed: {e}")))?;

    let asset0 = assets
        .get(0)
        .ok_or_else(|| ProvisionError::Invalid("no adoptium assets returned".into()))?;

    // Find the best matching binary entry (defensive in case API returns extras).
    let bin = asset0
        .binaries
        .iter()
        .find(|b| {
            b.architecture == arch
                && b.os == os
                && b.image_type == "jdk"
                && b.jvm_impl == "hotspot"
        })
        .ok_or_else(|| {
            ProvisionError::Invalid(format!(
                "no matching adoptium binary found for {major} {os} {arch} jdk hotspot"
            ))
        })?;

    let download_url = &bin.package.link;
    let expected_sha256 = normalize_hex(&bin.package.checksum).ok_or_else(|| {
        ProvisionError::Invalid(format!(
            "invalid checksum format from API: {}",
            bin.package.checksum
        ))
    })?;

    // 2) Download while hashing, then write to dest.
    let mut resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| ProvisionError::Invalid(format!("java download failed: {e}")))?
        .error_for_status()
        .map_err(|e| ProvisionError::Invalid(format!("java download failed: {e}")))?;

    // Write to a temp file first so we don't leave a corrupt dest on mismatch.
    let tmp_path = dest.with_extension("tmp");
    let mut file = tokio::fs::File::create(&tmp_path).await?;

    let mut hasher = Sha256::new();

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| ProvisionError::Invalid(format!("java download failed: {e}")))? {
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }

    file.flush().await?;

    let actual = hasher.finalize();
    let actual_hex = hex::encode(actual);

    // 3) Verify checksum
    if actual_hex != expected_sha256 {
        // Clean up temp file
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(ProvisionError::Invalid(format!(
            "sha256 mismatch: expected {expected_sha256}, got {actual_hex}"
        )));
    }

    // Atomically move into place
    tokio::fs::rename(&tmp_path, dest).await?;

    Ok(())
}

/// Normalize sha256 hex: lowercase, strip whitespace.
fn normalize_hex(s: &str) -> Option<String> {
    let t = s.trim().to_ascii_lowercase();
    if t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(t)
    } else {
        None
    }
}

fn extract_jdk_tar(archive_path: &Path, install_dir: &Path) -> Result<(), ProvisionError> {
    let parent = install_dir
        .parent()
        .ok_or_else(|| ProvisionError::Invalid("missing java install parent".to_string()))?;
    let install_name = install_dir
        .file_name()
        .ok_or_else(|| ProvisionError::Invalid("missing java install dir name".to_string()))?
        .to_string_lossy();
    let staging = parent.join(format!("{}.staging", install_name));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let tar_gz = fs::File::open(archive_path)?;
    let tar = GzDecoder::new(std::io::BufReader::new(tar_gz));
    let mut archive = tar::Archive::new(tar);
    archive.unpack(&staging)?;

    let extracted = fs::read_dir(&staging)?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .ok_or_else(|| ProvisionError::Invalid("failed to locate extracted jdk".to_string()))?;

    if install_dir.exists() {
        fs::remove_dir_all(install_dir)?;
    }
    fs::rename(extracted, install_dir)?;
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    Ok(())
}

fn java_bin_path(install_dir: &Path) -> PathBuf {
    if std::env::consts::OS == "macos" {
        return install_dir
            .join("Contents")
            .join("Home")
            .join("bin")
            .join("java");
    }
    install_dir.join("bin").join("java")
}
