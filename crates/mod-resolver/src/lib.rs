mod curseforge;
mod curseforge_proxy;
mod modrinth;
pub mod pointer;

#[cfg(feature = "blocking")]
use anyhow::Context;
use anyhow::{Result, bail};

pub use protocol::config::mods::{ModEntry, ModHashes, ModMetadata};

#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub project_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub project_id: String,
    pub desired_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedMod {
    pub entry: ModEntry,
    pub dependencies: Vec<ResolvedDependency>,
}

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
    let candidates = search(
        provider,
        query,
        loader,
        minecraft_version,
        normalized_pack_type,
        0,
        1,
    )
    .await?;
    let candidate = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("No {} results found for '{}'.", provider.label(), query))?;
    let resolved = match provider {
        Provider::Modrinth => {
            modrinth::resolve_by_project_id(
                &client,
                &candidate.project_id,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await?
        }
        Provider::CurseForge => {
            curseforge::resolve_by_project_id(
                &client,
                &candidate.project_id,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await?
        }
    };

    Ok(resolved.entry)
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
    let candidates = curseforge_proxy::search(
        &client,
        proxy_base_url,
        access_token,
        query,
        loader,
        minecraft_version,
        normalized_pack_type,
        0,
        1,
    )
    .await?;
    let candidate = candidates
        .first()
        .ok_or_else(|| anyhow::anyhow!("No CurseForge results found for '{}'.", query))?;

    let resolved = curseforge_proxy::resolve_by_project_id(
        &client,
        proxy_base_url,
        access_token,
        &candidate.project_id,
        loader,
        minecraft_version,
        desired_version,
        normalized_pack_type,
    )
    .await?;
    Ok(resolved.entry)
}

pub async fn search(
    provider: Provider,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();

    match provider {
        Provider::Modrinth => {
            modrinth::search(
                &client,
                query,
                loader,
                minecraft_version,
                normalized_pack_type,
                offset,
                limit,
            )
            .await
        }
        Provider::CurseForge => {
            curseforge::search(
                &client,
                query,
                loader,
                minecraft_version,
                normalized_pack_type,
                offset,
                limit,
            )
            .await
        }
    }
}

pub async fn search_curseforge_via_proxy(
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();
    curseforge_proxy::search(
        &client,
        proxy_base_url,
        access_token,
        query,
        loader,
        minecraft_version,
        normalized_pack_type,
        offset,
        limit,
    )
    .await
}

pub async fn resolve_by_project_id(
    provider: Provider,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();

    match provider {
        Provider::Modrinth => {
            modrinth::resolve_by_project_id(
                &client,
                project_id,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await
        }
        Provider::CurseForge => {
            curseforge::resolve_by_project_id(
                &client,
                project_id,
                loader,
                minecraft_version,
                desired_version,
                normalized_pack_type,
            )
            .await
        }
    }
}

pub async fn resolve_curseforge_by_project_id_via_proxy(
    proxy_base_url: &str,
    access_token: &str,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let normalized_pack_type = normalize_pack_type(pack_type)?;
    let client = reqwest::Client::new();
    curseforge_proxy::resolve_by_project_id(
        &client,
        proxy_base_url,
        access_token,
        project_id,
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

#[cfg(feature = "blocking")]
pub fn search_blocking(
    provider: Provider,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(search(
        provider,
        query,
        loader,
        minecraft_version,
        pack_type,
        offset,
        limit,
    ))
}

#[cfg(feature = "blocking")]
pub fn search_curseforge_via_proxy_blocking(
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(search_curseforge_via_proxy(
        proxy_base_url,
        access_token,
        query,
        loader,
        minecraft_version,
        pack_type,
        offset,
        limit,
    ))
}

#[cfg(feature = "blocking")]
pub fn resolve_by_project_id_blocking(
    provider: Provider,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(resolve_by_project_id(
        provider,
        project_id,
        loader,
        minecraft_version,
        desired_version,
        pack_type,
    ))
}

#[cfg(feature = "blocking")]
pub fn resolve_curseforge_by_project_id_via_proxy_blocking(
    proxy_base_url: &str,
    access_token: &str,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let runtime = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    runtime.block_on(resolve_curseforge_by_project_id_via_proxy(
        proxy_base_url,
        access_token,
        project_id,
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
        "other" => Ok("other"),
        other => bail!("Unsupported pack type: {}", other),
    }
}

impl Provider {
    fn label(self) -> &'static str {
        match self {
            Provider::Modrinth => "Modrinth",
            Provider::CurseForge => "CurseForge",
        }
    }
}
