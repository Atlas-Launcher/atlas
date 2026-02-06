mod curseforge;
mod curseforge_proxy;
mod modrinth;

#[cfg(feature = "blocking")]
use anyhow::Context;
use anyhow::{Result, bail};

pub use protocol::config::mods::{ModEntry, ModHashes, ModMetadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Modrinth,
    CurseForge,
}

impl Provider {
    pub fn from_short_code(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "mr" | "modrinth" => Some(Self::Modrinth),
            "cf" | "curseforge" => Some(Self::CurseForge),
            _ => None,
        }
    }
}

pub async fn resolve(
    provider: Provider,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();

    match provider {
        Provider::Modrinth => {
            modrinth::resolve(
                &client,
                query,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await
        }
        Provider::CurseForge => {
            curseforge::resolve(
                &client,
                query,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await
        }
    }
}

pub async fn resolve_curseforge_via_proxy(
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();
    curseforge_proxy::resolve(
        &client,
        proxy_base_url,
        access_token,
        query,
        loader,
        minecraft_version,
        desired_version,
        normalized_pack_type,
    )
    .await
}

#[cfg(feature = "blocking")]
pub fn resolve_blocking(
    provider: Provider,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(resolve(
        provider,
        query,
        loader,
        minecraft_version,
        desired_version,
        pack_type,
    ))
}

#[cfg(feature = "blocking")]
pub fn resolve_curseforge_via_proxy_blocking(
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(resolve_curseforge_via_proxy(
        proxy_base_url,
        access_token,
        query,
        loader,
        minecraft_version,
        desired_version,
        pack_type,
    ))
}

fn normalize_pack_type(pack_type: &str) -> Result<&'static str> {
    let normalized = pack_type.trim().to_lowercase();
    match normalized.as_str() {
        "mod" => Ok("mod"),
        "shader" => Ok("shader"),
        "resourcepack" => Ok("resourcepack"),
        other => bail!("Unsupported pack type: {}", other),
    }
}
