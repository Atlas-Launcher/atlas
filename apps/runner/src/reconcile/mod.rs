use anyhow::{Result, Context};
use std::path::PathBuf;
use std::sync::Arc;
use crate::hub::HubClient;
use crate::fetch::Fetcher;
use crate::cache::Cache;
use crate::assemble::Assembler;
use crate::hub::BuildBlobResult;
use crate::backup;
use crate::hub::whitelist::InstanceConfig;
use protocol::config::atlas::parse_config;
// PackBlob removed

pub struct Reconciler {
    hub: Arc<HubClient>,
    fetcher: Arc<Fetcher>,
    cache: Arc<Cache>,
    base_dir: PathBuf,
}

impl Reconciler {
    pub fn new(hub: Arc<HubClient>, fetcher: Arc<Fetcher>, cache: Arc<Cache>, base_dir: PathBuf) -> Self {
        Self { hub, fetcher, cache, base_dir }
    }

    pub async fn reconcile(&self, pack_id: &str, channel: &str) -> Result<()> {
        println!("Reconciling instance for pack: {} (channel: {})", pack_id, channel);

        // 1. Fetch latest blob
        println!("Downloading pack build from Hub...");
        let build = self.hub.get_build_blob(pack_id, channel).await
            .context("Failed to fetch build blob")?;
        
        // 2. Decode blob
        println!("Decoding pack build...");
        let blob = protocol::decode_blob(&build.bytes)
            .context("Failed to decode build blob")?;

        println!(
            "Minecraft {} with {} loader.",
            blob.metadata.minecraft_version,
            format_loader(blob.metadata.loader)
        );

        let reinstall_required = should_force_reinstall(&self.base_dir, &blob, &build).await?;
        if reinstall_required {
            println!("Full reinstall required. Archiving world directories...");
        }

        write_pack_metadata(&self.base_dir, &blob).await?;
        update_instance_metadata(&self.base_dir, &blob).await?;

        // 3. Fetch artifacts from manifest
        let mut artifacts = Vec::new();
        for dep in &blob.manifest.dependencies {
            artifacts.push((dep.url.clone(), dep.hash.hex.clone()));
        }
        
        println!("Pulling {} mod artifacts...", artifacts.len());
        self.fetcher.fetch_multiple(artifacts).await?;

        // 4. Assemble runtime in staging area
        let staging_dir = self.base_dir.join("runtime/staging");
        let assembler = Assembler::new(staging_dir.clone());
        
        println!("Writing server files and configs...");
        assembler.assemble(&blob).await?;

        println!("Installing loader and linking mods...");
        assembler.link_artifacts(&self.cache.get_path(""), &blob.manifest).await?;

        // 5. Finalize (Stop server, Swap, Start server)
        // This will be implemented when Supervisor is ready
        self.finalize(&staging_dir, reinstall_required).await?;

        Ok(())
    }

    async fn finalize(&self, staging_dir: &PathBuf, reinstall_required: bool) -> Result<()> {
        let current_dir = self.base_dir.join("runtime/current");
        
        println!("Finalizing deployment (atomic swap)...");
        
        // TODO: Stop server here if running
        
        if current_dir.exists() {
            if reinstall_required {
                let archive_dir = self.base_dir.join("runtime/world-archive");
                let _ = backup::archive_worlds(
                    &current_dir,
                    &archive_dir,
                    "worlds",
                    0,
                )
                .await?;
            } else {
                preserve_server_files(&current_dir, staging_dir).await?;
            }
            // Move current to old or delete it
            let old_dir = self.base_dir.join("runtime/old");
            if old_dir.exists() {
                tokio::fs::remove_dir_all(&old_dir).await?;
            }
            tokio::fs::rename(&current_dir, &old_dir).await?;
        }

        tokio::fs::rename(staging_dir, &current_dir).await?;
        
        // TODO: Start server here

        Ok(())
    }
}

async fn preserve_server_files(current_dir: &PathBuf, staging_dir: &PathBuf) -> Result<()> {
    let files = [
        "run.sh",
        "server.jar",
        "fabric-server-launch.jar",
        "user_jvm_args.txt",
        "unix_args.txt",
    ];

    for name in files {
        let src = current_dir.join(name);
        let dest = staging_dir.join(name);
        if src.exists() {
            if let Some(parent) = dest.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            let _ = tokio::fs::copy(&src, &dest).await;
        }
    }

    let libraries = current_dir.join("libraries");
    if libraries.exists() {
        let dest = staging_dir.join("libraries");
        copy_dir_recursive(&libraries, &dest).await?;
    }

    Ok(())
}

async fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    let mut stack = vec![(src.clone(), dest.clone())];

    while let Some((current_src, current_dest)) = stack.pop() {
        tokio::fs::create_dir_all(&current_dest).await?;
        let mut entries = tokio::fs::read_dir(&current_src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;
            let dest_path = current_dest.join(entry.file_name());
            if file_type.is_dir() {
                stack.push((path, dest_path));
            } else if file_type.is_file() {
                let _ = tokio::fs::copy(&path, &dest_path).await;
            }
        }
    }
    Ok(())
}

async fn write_pack_metadata(base_dir: &PathBuf, blob: &protocol::PackBlob) -> Result<()> {
    let meta_dir = base_dir.join("runtime/current");
    tokio::fs::create_dir_all(&meta_dir).await?;
    let meta_path = meta_dir.join("pack-meta.json");
    let payload = serde_json::json!({
        "minecraftVersion": blob.metadata.minecraft_version,
        "loader": format_loader(blob.metadata.loader),
    });
    let content = serde_json::to_string_pretty(&payload)?;
    tokio::fs::write(meta_path, content).await?;
    Ok(())
}

async fn update_instance_metadata(base_dir: &PathBuf, blob: &protocol::PackBlob) -> Result<()> {
    let instance_path = base_dir.join("instance.toml");
    let mut config = InstanceConfig::load(&instance_path).await
        .context("Missing instance.toml while updating metadata")?;
    config.minecraft_version = Some(blob.metadata.minecraft_version.clone());
    config.modloader = Some(format_loader(blob.metadata.loader).to_ascii_lowercase());

    let atlas_path = base_dir.join("runtime/current/atlas.toml");
    if let Ok(contents) = tokio::fs::read_to_string(&atlas_path).await {
        if let Ok(atlas_config) = parse_config(&contents) {
            config.modloader = Some(atlas_config.versions.modloader.to_ascii_lowercase());
            config.modloader_version = Some(atlas_config.versions.modloader_version);
        }
    }
    config.save(&instance_path).await?;
    Ok(())
}

async fn should_force_reinstall(
    base_dir: &PathBuf,
    blob: &protocol::PackBlob,
    build: &BuildBlobResult,
) -> Result<bool> {
    if build.force_reinstall || build.requires_full_reinstall {
        return Ok(true);
    }

    let meta_path = base_dir.join("runtime/current/pack-meta.json");
    let content = match tokio::fs::read_to_string(&meta_path).await {
        Ok(content) => content,
        Err(_) => return Ok(false),
    };

    let meta: serde_json::Value = serde_json::from_str(&content)?;
    let prev_mc = meta.get("minecraftVersion").and_then(|value| value.as_str());
    let prev_loader = meta.get("loader").and_then(|value| value.as_str());

    let new_loader = format_loader(blob.metadata.loader);
    let loader_changed = prev_loader.map(|value| value != new_loader).unwrap_or(false);
    let mc_changed = prev_mc
        .map(|value| value != blob.metadata.minecraft_version)
        .unwrap_or(false);

    Ok(loader_changed || mc_changed)
}


fn format_loader(loader: protocol::Loader) -> &'static str {
    match loader {
        protocol::Loader::Fabric => "Fabric",
        protocol::Loader::Forge => "Forge",
        protocol::Loader::Neo => "NeoForge",
    }
}
