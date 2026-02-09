use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

const JAVA_DIR: &str = "/var/lib/atlas-runner/java";

pub async fn ensure_java_for_minecraft(mc_version: &str, override_major: Option<u32>) -> Result<PathBuf> {
    let minimum = java_version_for_minecraft(mc_version);
    let major = match override_major {
        Some(value) if value >= minimum => value,
        Some(_) => minimum,
        None => minimum,
    };
    let install_dir = PathBuf::from(JAVA_DIR).join(format!("jdk-{major}"));
    let java_bin = install_dir.join("bin/java");
    if java_bin.exists() {
        return Ok(java_bin);
    }

    tokio::fs::create_dir_all(&install_dir)
        .await
        .context("Failed to create java install dir")?;

    let archive_path = install_dir.join("download.tar.gz");
    download_jdk_archive(major, &archive_path).await?;

    let install_dir_clone = install_dir.clone();
    let archive_clone = archive_path.clone();
    tokio::task::spawn_blocking(move || extract_jdk(&archive_clone, &install_dir_clone))
        .await
        .context("Failed to join java install task")??;

    let _ = tokio::fs::remove_file(&archive_path).await;

    if !java_bin.exists() {
        anyhow::bail!("Java install failed: {} not found", java_bin.display());
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

async fn download_jdk_archive(major: u32, dest: &Path) -> Result<()> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => anyhow::bail!("Unsupported architecture: {other}"),
    };

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/linux/{arch}/jdk/hotspot/normal/eclipse"
    );
    let response = reqwest::get(url).await?.error_for_status()?;
    let bytes = response.bytes().await?;
    tokio::fs::write(dest, &bytes).await?;
    Ok(())
}

fn extract_jdk(archive_path: &Path, install_dir: &Path) -> Result<()> {
    let parent = install_dir
        .parent()
        .context("Missing java install parent dir")?;
    let install_name = install_dir
        .file_name()
        .context("Missing java install dir name")?
        .to_string_lossy();
    let staging = parent.join(format!("{}.staging", install_name));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let tar_gz = File::open(archive_path)?;
    let tar = GzDecoder::new(BufReader::new(tar_gz));
    let mut archive = tar::Archive::new(tar);
    archive.unpack(&staging)?;

    let extracted = fs::read_dir(&staging)?
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .context("Failed to locate extracted JDK directory")?;

    if install_dir.exists() {
        fs::remove_dir_all(install_dir)?;
    }
    fs::rename(extracted, install_dir)?;
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    Ok(())
}
