use crate::launcher::error::LauncherError;
use crate::launcher::manifest::VersionData;
use crate::models::FabricLoaderVersion;
use crate::net::http::{fetch_json, HttpError};
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct FabricLoaderEntry {
    loader: FabricLoaderInfo,
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: bool,
}

pub async fn fetch_loader_versions(
    client: &Client,
    minecraft_version: &str,
) -> Result<Vec<FabricLoaderVersion>, HttpError> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{minecraft_version}");
    let entries: Vec<FabricLoaderEntry> = fetch_json(client, &url).await?;
    Ok(entries
        .into_iter()
        .map(|entry| FabricLoaderVersion {
            version: entry.loader.version,
            stable: entry.loader.stable,
        })
        .collect())
}

pub async fn fetch_profile(
    client: &Client,
    minecraft_version: &str,
    requested: Option<String>,
) -> Result<VersionData, LauncherError> {
    let loader_version = resolve_loader_version(client, minecraft_version, requested).await?;
    let profile_url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{minecraft_version}/{loader_version}/profile/json"
    );
    Ok(fetch_json::<VersionData>(client, &profile_url).await?)
}

async fn resolve_loader_version(
    client: &Client,
    minecraft_version: &str,
    requested: Option<String>,
) -> Result<String, LauncherError> {
    if let Some(version) = requested {
        if !version.trim().is_empty() {
            return Ok(version.trim().to_string());
        }
    }

    let entries = fetch_loader_versions(client, minecraft_version).await?;
    let chosen = entries
        .iter()
        .find(|entry| entry.stable)
        .or_else(|| entries.first())
        .ok_or_else(|| "No Fabric loader versions found.".to_string())?;
    Ok(chosen.version.clone())
}
