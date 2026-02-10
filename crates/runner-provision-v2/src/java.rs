use std::fs;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;

use crate::errors::ProvisionError;

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
        return Ok(java_bin);
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

async fn download_jdk_archive(major: u32, dest: &Path) -> Result<(), ProvisionError> {
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

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );
    let response = reqwest::get(url)
        .await
        .map_err(|err| ProvisionError::Invalid(format!("java download failed: {err}")))?
        .error_for_status()
        .map_err(|err| ProvisionError::Invalid(format!("java download failed: {err}")))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|err| ProvisionError::Invalid(format!("java download failed: {err}")))?;
    tokio::fs::write(dest, &bytes).await?;
    Ok(())
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
