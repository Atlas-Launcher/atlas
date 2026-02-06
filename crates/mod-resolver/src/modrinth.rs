use anyhow::{Context, Result, bail};
use serde::Deserialize;

use protocol::config::mods::{ModEntry, ModHashes, ModMetadata};

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    project_id: String,
    title: Option<String>,
}

#[derive(Deserialize)]
struct VersionInfo {
    name: String,
    version_number: String,
    files: Vec<ModFile>,
}

#[derive(Deserialize)]
struct ModFile {
    url: String,
    hashes: ModHashes,
}

pub async fn resolve(
    client: &reqwest::Client,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let facets = format!(
        "[[\"project_type:{}\"],[\"categories:{}\"],[\"versions:{}\"]]",
        pack_type, loader, minecraft_version
    );
    let search_url = reqwest::Url::parse_with_params(
        "https://api.modrinth.com/v2/search",
        [
            ("query", query),
            ("limit", "1"),
            ("facets", facets.as_str()),
        ],
    )
    .context("Failed to build Modrinth search URL")?;
    let search = client
        .get(search_url)
        .send()
        .await
        .context("Modrinth search failed")?
        .error_for_status()
        .context("Modrinth search returned an error")?
        .json::<SearchResponse>()
        .await
        .context("Failed to parse Modrinth search response")?;

    let hit = search.hits.first().context("No Modrinth results found")?;

    let version_url = format!(
        "https://api.modrinth.com/v2/project/{}/version?loaders=[\"{}\"]&game_versions=[\"{}\"]",
        hit.project_id, loader, minecraft_version
    );
    let versions = client
        .get(version_url)
        .send()
        .await
        .context("Failed to load Modrinth versions")?
        .error_for_status()
        .context("Modrinth versions returned an error")?
        .json::<Vec<VersionInfo>>()
        .await
        .context("Failed to parse Modrinth versions")?;

    let version = if let Some(desired) = desired_version {
        versions
            .iter()
            .find(|item| item.version_number == desired || item.name == desired)
            .context("Requested version not found")?
    } else {
        versions.first().context("No Modrinth versions found")?
    };

    let file = version
        .files
        .first()
        .context("No Modrinth files found for version")?;

    if file.url.trim().is_empty() {
        bail!("Modrinth did not return a downloadable URL.");
    }

    Ok(ModEntry {
        metadata: Some(ModMetadata {
            name: hit.title.clone().unwrap_or_else(|| query.to_string()),
        }),
        source: "modrinth".to_string(),
        project_id: hit.project_id.clone(),
        version: version.version_number.clone(),
        file_id: None,
        download_url: Some(file.url.clone()),
        hashes: Some(ModHashes {
            sha1: None,
            sha512: file.hashes.sha512.clone(),
        }),
    })
}
