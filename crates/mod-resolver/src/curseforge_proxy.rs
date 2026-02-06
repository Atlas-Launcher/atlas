use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{ModEntry, ModHashes, ModMetadata};

const GAME_ID_MINECRAFT: i32 = 432;

#[derive(Deserialize)]
struct CfResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct CfDownloadUrlResponse {
    data: String,
}

#[derive(Deserialize)]
struct CfMod {
    id: i64,
    name: String,
}

#[derive(Deserialize)]
struct CfFile {
    id: i64,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
    hashes: Vec<CfHash>,
}

#[derive(Deserialize)]
struct CfHash {
    algo: i32,
    value: String,
}

pub async fn resolve(
    client: &reqwest::Client,
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    _desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let class_id = class_id_for(pack_type)?;
    let loader_id = loader_id_for(loader)?;
    let base = format!("{}/api/curseforge", proxy_base_url.trim_end_matches('/'));

    let search_url = reqwest::Url::parse_with_params(
        &format!("{base}/mods/search"),
        [
            ("gameId", GAME_ID_MINECRAFT.to_string()),
            ("searchFilter", query.to_string()),
            ("pageSize", "1".to_string()),
            ("classId", class_id.to_string()),
            ("modLoaderType", loader_id.to_string()),
            ("gameVersion", minecraft_version.to_string()),
        ],
    )
    .context("Failed to build CurseForge proxy search URL")?;

    let mods = client
        .get(search_url)
        .bearer_auth(access_token)
        .send()
        .await
        .context("CurseForge proxy search failed")?
        .error_for_status()
        .context("CurseForge proxy search returned an error")?
        .json::<CfResponse<CfMod>>()
        .await
        .context("Failed to parse CurseForge proxy search response")?;

    let mod_entry = mods.data.first().context("No CurseForge results found")?;

    let files_url = reqwest::Url::parse_with_params(
        &format!("{base}/mods/{}/files", mod_entry.id),
        [
            ("gameVersion", minecraft_version.to_string()),
            ("modLoaderType", loader_id.to_string()),
            ("pageSize", "1".to_string()),
        ],
    )
    .context("Failed to build CurseForge proxy files URL")?;
    let files = client
        .get(files_url)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Failed to load CurseForge proxy files")?
        .error_for_status()
        .context("CurseForge proxy files returned an error")?
        .json::<CfResponse<CfFile>>()
        .await
        .context("Failed to parse CurseForge proxy files response")?;

    let file = files.data.first().context("No CurseForge files found")?;

    let download_url = if let Some(url) = file.download_url.clone().filter(|v| !v.trim().is_empty())
    {
        url
    } else {
        let url = format!(
            "{base}/mods/{}/files/{}/download-url",
            mod_entry.id, file.id
        );
        let response = client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await
            .context("Failed to load CurseForge proxy download URL")?
            .error_for_status()
            .context("CurseForge proxy download URL returned an error")?
            .json::<CfDownloadUrlResponse>()
            .await
            .context("Failed to parse CurseForge proxy download URL response")?;
        let trimmed = response.data.trim();
        if trimmed.is_empty() {
            bail!("CurseForge did not return a downloadable URL.");
        }
        trimmed.to_string()
    };

    let sha1 = file
        .hashes
        .iter()
        .find(|hash| hash.algo == 1)
        .map(|hash| hash.value.clone());

    Ok(ModEntry {
        metadata: Some(ModMetadata {
            name: mod_entry.name.clone(),
        }),
        source: "curseforge".to_string(),
        project_id: mod_entry.id.to_string(),
        version: file.display_name.clone(),
        file_id: Some(file.id.to_string()),
        download_url: Some(download_url),
        hashes: Some(ModHashes { sha1, sha512: None }),
    })
}

fn loader_id_for(loader: &str) -> Result<i32> {
    match loader.to_lowercase().as_str() {
        "fabric" => Ok(4),
        "forge" => Ok(1),
        "neo" | "neoforge" => Ok(6),
        other => bail!("Unsupported loader for CurseForge: {}", other),
    }
}

fn class_id_for(pack_type: &str) -> Result<i32> {
    match pack_type {
        "mod" => Ok(6),
        "shader" => Ok(6552),
        "resourcepack" => Ok(12),
        other => bail!("Unsupported pack type for CurseForge: {}", other),
    }
}
