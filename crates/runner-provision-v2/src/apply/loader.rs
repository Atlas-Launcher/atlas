use std::path::{Path, PathBuf};

use protocol::{Loader, PackMetadata};

use crate::{errors::ProvisionError, now_millis};
use std::process::Output;

const FABRIC_INSTALLER_META: &str = "https://meta.fabricmc.net/v2/versions/installer";
const FABRIC_INSTALLER_MAVEN: &str = "https://maven.fabricmc.net/net/fabricmc/fabric-installer";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";

pub async fn ensure_loader_installed(
    server_root: &Path,
    staging_current: &Path,
    meta: &PackMetadata,
    java_bin: &Path,
) -> Result<(), ProvisionError> {
    if has_server_launch_files(staging_current).await {
        return Ok(());
    }

    let cache_dir = server_root.join(".runner").join("cache").join("loader");
    tokio::fs::create_dir_all(&cache_dir).await?;

    match meta.loader {
        Loader::Fabric => {
            let installer_version = resolve_fabric_installer_version().await?;
            let installer_url = format!(
                "{FABRIC_INSTALLER_MAVEN}/{installer_version}/fabric-installer-{installer_version}.jar"
            );
            let installer_cache = cache_dir.join(format!(
                "fabric-installer-{installer_version}.jar"
            ));
            download_to_path(&installer_url, &installer_cache).await?;

            let installer_path = staging_current.join("fabric-installer.jar");
            copy_if_missing(&installer_cache, &installer_path).await?;
            run_fabric_installer(
                staging_current,
                &installer_path,
                meta,
                java_bin,
                server_root,
            )
            .await?;
        }
        Loader::Forge => {
            let loader_version = require_loader_version(meta, "forge")?;
            let installer_url = format!(
                "{FORGE_MAVEN}/{mc}-{loader}/forge-{mc}-{loader}-installer.jar",
                mc = meta.minecraft_version,
                loader = loader_version
            );
            let installer_cache = cache_dir.join(format!(
                "forge-installer-{}-{}.jar",
                meta.minecraft_version, loader_version
            ));
            download_to_path(&installer_url, &installer_cache).await?;

            let installer_path = staging_current.join("forge-installer.jar");
            copy_if_missing(&installer_cache, &installer_path).await?;
            run_server_installer(
                staging_current,
                &installer_path,
                java_bin,
                server_root,
                "forge",
            )
            .await?;
        }
        Loader::Neo => {
            let loader_version = require_loader_version(meta, "neoforge")?;
            let installer_url = format!(
                "{NEOFORGE_MAVEN}/{loader}/neoforge-{loader}-installer.jar",
                loader = loader_version
            );
            let installer_cache = cache_dir.join(format!(
                "neoforge-installer-{loader}.jar",
                loader = loader_version
            ));
            download_to_path(&installer_url, &installer_cache).await?;

            let installer_path = staging_current.join("neoforge-installer.jar");
            copy_if_missing(&installer_cache, &installer_path).await?;
            run_server_installer(
                staging_current,
                &installer_path,
                java_bin,
                server_root,
                "neoforge",
            )
            .await?;
        }
    }

    if !has_server_launch_files(staging_current).await {
        return Err(ProvisionError::Invalid(
            "loader installer did not produce launch files".to_string(),
        ));
    }

    Ok(())
}

async fn has_server_launch_files(runtime_dir: &Path) -> bool {
    runtime_dir.join("run.sh").exists()
        || runtime_dir.join("fabric-server-launch.jar").exists()
        || runtime_dir.join("server.jar").exists()
}

fn require_loader_version(meta: &PackMetadata, loader: &str) -> Result<String, ProvisionError> {
    let value = meta.loader_version.trim();
    if value.is_empty() {
        return Err(ProvisionError::Invalid(format!(
            "missing loader_version for {loader}"
        )));
    }
    Ok(value.to_string())
}

async fn download_to_path(url: &str, dest: &PathBuf) -> Result<(), ProvisionError> {
    if let Ok(meta) = tokio::fs::metadata(dest).await {
        if meta.len() > 0 {
            return Ok(());
        }
    }

    let response = reqwest::get(url)
        .await
        .map_err(|err| ProvisionError::Invalid(format!("download failed: {err}")))?
        .error_for_status()
        .map_err(|err| ProvisionError::Invalid(format!("download failed: {err}")))?;
    let bytes = response
        .bytes()
        .await
        .map_err(|err| ProvisionError::Invalid(format!("download failed: {err}")))?;
    tokio::fs::write(dest, &bytes).await?;
    Ok(())
}

async fn copy_if_missing(source: &PathBuf, dest: &PathBuf) -> Result<(), ProvisionError> {
    if let Ok(meta) = tokio::fs::metadata(dest).await {
        if meta.len() > 0 {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::copy(source, dest).await?;
    Ok(())
}

async fn run_server_installer(
    runtime_dir: &Path,
    installer: &PathBuf,
    java_bin: &Path,
    server_root: &Path,
    label: &str,
) -> Result<(), ProvisionError> {
    let installer_arg = installer
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| installer.to_string_lossy().to_string());
    let cmd_line = format!(
        "{} -jar {} --installServer",
        java_bin.display(),
        installer_arg
    );
    let log_path = installer_log_path(server_root, label);
    let output = tokio::process::Command::new(java_bin)
        .current_dir(runtime_dir)
        .arg("-jar")
        .arg(installer_arg)
        .arg("--installServer")
        .output()
        .await;

    let output = match output {
        Ok(value) => {
            write_installer_log(&log_path, &cmd_line, &value).await;
            value
        }
        Err(err) => {
            write_installer_error_log(&log_path, &cmd_line, &err).await;
            return Err(err.into());
        }
    };

    if !output.status.success() {
        return Err(ProvisionError::Invalid(format!(
            "server installer failed with exit code: {} (log: {})",
            output.status,
            log_path.display()
        )));
    }
    Ok(())
}

async fn run_fabric_installer(
    runtime_dir: &Path,
    installer: &PathBuf,
    meta: &PackMetadata,
    java_bin: &Path,
    server_root: &Path,
) -> Result<(), ProvisionError> {
    let loader_version = require_loader_version(meta, "fabric")?;
    let installer_arg = installer
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| installer.to_string_lossy().to_string());

    let cmd_line = format!(
        "{} -jar {} server -mcversion {} -loader {} -downloadMinecraft",
        java_bin.display(),
        installer_arg,
        meta.minecraft_version,
        loader_version
    );
    let log_path = installer_log_path(server_root, "fabric");
    let output = tokio::process::Command::new(java_bin)
        .current_dir(runtime_dir)
        .arg("-jar")
        .arg(installer_arg)
        .arg("server")
        .arg("-mcversion")
        .arg(&meta.minecraft_version)
        .arg("-loader")
        .arg(&loader_version)
        .arg("-downloadMinecraft")
        .output()
        .await;

    let output = match output {
        Ok(value) => {
            write_installer_log(&log_path, &cmd_line, &value).await;
            value
        }
        Err(err) => {
            write_installer_error_log(&log_path, &cmd_line, &err).await;
            return Err(err.into());
        }
    };

    if !output.status.success() {
        return Err(ProvisionError::Invalid(format!(
            "fabric installer failed with exit code: {} (log: {})",
            output.status,
            log_path.display()
        )));
    }
    Ok(())
}

fn installer_log_path(server_root: &Path, label: &str) -> PathBuf {
    let logs_dir = server_root.join(".runner").join("logs");
    let stamp = now_millis();
    logs_dir.join(format!("installer-{label}-{stamp}.log"))
}

async fn write_installer_log(log_path: &Path, cmd_line: &str, output: &Output) {
    let mut contents = String::new();
    contents.push_str("command: ");
    contents.push_str(cmd_line);
    contents.push('\n');
    contents.push_str("status: ");
    contents.push_str(&output.status.to_string());
    contents.push('\n');
    contents.push_str("stdout:\n");
    contents.push_str(&String::from_utf8_lossy(&output.stdout));
    contents.push_str("\n\nstderr:\n");
    contents.push_str(&String::from_utf8_lossy(&output.stderr));
    contents.push('\n');

    if let Some(parent) = log_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(log_path, contents).await;
}

async fn write_installer_error_log(log_path: &Path, cmd_line: &str, err: &std::io::Error) {
    let mut contents = String::new();
    contents.push_str("command: ");
    contents.push_str(cmd_line);
    contents.push('\n');
    contents.push_str("error: ");
    contents.push_str(&err.to_string());
    contents.push('\n');

    if let Some(parent) = log_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(log_path, contents).await;
}

async fn resolve_fabric_installer_version() -> Result<String, ProvisionError> {
    #[derive(serde::Deserialize)]
    struct FabricInstallerEntry {
        version: String,
        stable: Option<bool>,
    }

    let entries = reqwest::get(FABRIC_INSTALLER_META)
        .await
        .map_err(|err| ProvisionError::Invalid(format!("fabric installer lookup failed: {err}")))?
        .error_for_status()
        .map_err(|err| ProvisionError::Invalid(format!("fabric installer lookup failed: {err}")))?
        .json::<Vec<FabricInstallerEntry>>()
        .await
        .map_err(|err| ProvisionError::Invalid(format!("fabric installer lookup failed: {err}")))?;

    let chosen = entries
        .iter()
        .find(|entry| entry.stable.unwrap_or(false))
        .or_else(|| entries.first())
        .ok_or_else(|| ProvisionError::Invalid("no fabric installer versions found".into()))?;

    Ok(chosen.version.clone())
}
