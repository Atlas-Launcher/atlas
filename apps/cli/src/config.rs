use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use atlas_auth::device_code::{DEFAULT_ATLAS_HUB_URL, normalize_hub_url};
use protocol::config::atlas::AtlasConfig;
use protocol::pack::{BuildInput, BuildOutput, build_pack_bytes as build_binary};

use crate::io;

pub struct CliSettings {
    pub pack_id: Option<String>,
    pub hub_url: String,
    pub channel: String,
}

pub fn load_atlas_config(root: &Path) -> Result<AtlasConfig> {
    let config_path = root.join("atlas.toml");

    if !config_path.exists() {
        bail!("atlas.toml not found at {}", config_path.display());
    }

    let config_text = io::read_to_string(&config_path)?;
    let config = protocol::config::atlas::parse_config(&config_text)
        .map_err(|_| anyhow::anyhow!("Failed to parse atlas.toml"))?;
    Ok(config)
}

pub fn resolve_cli_settings(
    root: &Path,
    pack_id_override: Option<String>,
    hub_url_override: Option<String>,
    channel_override: Option<String>,
) -> Result<CliSettings> {
    let config = load_atlas_config(root)?;
    let cli_config = config.cli.clone().unwrap_or_default();
    let pack_id = normalize_optional(pack_id_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_PACK_ID").ok()))
        .or_else(|| normalize_optional(cli_config.pack_id));

    let hub_url = normalize_optional(hub_url_override)
        .or_else(|| normalize_optional(std::env::var("ATLAS_HUB_URL").ok()))
        .or_else(|| normalize_optional(cli_config.hub_url))
        .unwrap_or_else(|| DEFAULT_ATLAS_HUB_URL.to_string());

    let channel = normalize_optional(channel_override)
        .or_else(|| normalize_optional(cli_config.default_channel))
        .unwrap_or_else(|| "dev".to_string());

    Ok(CliSettings {
        pack_id,
        hub_url: normalize_hub_url(&hub_url),
        channel,
    })
}

pub fn build_pack_bytes(
    root: &Path,
    pack_id_arg: Option<String>,
    version_override: Option<String>,
    zstd_level: i32,
) -> Result<BuildOutput> {
    let config = load_atlas_config(root)?;
    let pack_id = normalize_optional(pack_id_arg)
        .or_else(|| normalize_optional(config.cli.as_ref().and_then(|cli| cli.pack_id.clone())))
        .context("pack_id is required (pass --pack-id or set pack_id in atlas.toml)")?;

    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    io::insert_file(&mut files, root, "atlas.toml")?;
    io::insert_file(&mut files, root, "mods.toml")?;
    io::insert_config_dir(&mut files, root)?;

    let build = build_binary(
        BuildInput {
            pack_id,
            config,
            files,
            version_override,
        },
        zstd_level,
    )
    .map_err(anyhow::Error::from)
    .context("Failed to encode pack")?;
    Ok(BuildOutput {
        bytes: build.bytes,
        metadata: build.metadata,
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|val| {
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
