use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use url::form_urlencoded::Serializer;

use super::{ModEntry, ModHashes};

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    project_id: String,
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

#[derive(Deserialize)]
struct ModHashes {
    sha1: String,
    sha512: String,
}

pub fn resolve(
    query: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let client = Client::new();
    let facets = format!(
        "[[\"project_type:{}\"],[\"categories:{}\"],[\"versions:{}\"]]",
        pack_type, loader, minecraft_version
    );
    let mut serializer = Serializer::new(String::new());
    serializer.append_pair("query", query);
    serializer.append_pair("limit", "1");
    serializer.append_pair("facets", &facets);
    let url = format!("https://api.modrinth.com/v2/search?{}", serializer.finish());

    let search = client
        .get(url)
        .send()
        .context("Modrinth search failed")?
        .error_for_status()
        .context("Modrinth search returned an error")?
        .json::<SearchResponse>()
        .context("Failed to parse Modrinth search response")?;

    let hit = search.hits.first().context("No Modrinth results found")?;

    let version_url = format!(
        "https://api.modrinth.com/v2/project/{}/version?loaders=[\"{}\"]&game_versions=[\"{}\"]",
        hit.project_id, loader, minecraft_version
    );
    let versions = client
        .get(version_url)
        .send()
        .context("Failed to load Modrinth versions")?
        .error_for_status()
        .context("Modrinth versions returned an error")?
        .json::<Vec<VersionInfo>>()
        .context("Failed to parse Modrinth versions")?;

    let version = if let Some(desired) = desired_version {
        versions
            .iter()
            .find(|item| item.version_number == desired || item.name == desired)
            .cloned()
            .context("Requested version not found")?
    } else {
        versions.first().cloned().context("No Modrinth versions found")?
    };

    let file = version
        .files
        .first()
        .context("No Modrinth files found for version")?;

    if file.url.trim().is_empty() {
        bail!("Modrinth did not return a downloadable URL.");
    }

    Ok(ModEntry {
        source: "modrinth".to_string(),
        project_id: hit.project_id.clone(),
        version: version.version_number.clone(),
        file_id: None,
        download_url: Some(file.url.clone()),
        hashes: Some(ModHashes {
            sha1: None,
            sha512: Some(file.hashes.sha512.clone()),
        }),
    })
}
