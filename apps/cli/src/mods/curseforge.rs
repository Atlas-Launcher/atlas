use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use url::form_urlencoded::Serializer;

use super::{ModEntry, ModHashes};

const GAME_ID_MINECRAFT: i32 = 432;

#[derive(Deserialize)]
struct CfResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct CfMod {
    id: i64,
    name: String,
    slug: String,
}

#[derive(Deserialize)]
struct CfFile {
    id: i64,
    displayName: String,
    downloadUrl: Option<String>,
    hashes: Vec<CfHash>,
}

#[derive(Deserialize)]
struct CfHash {
    algo: i32,
    value: String,
}

pub fn resolve(
    query: &str,
    loader: &str,
    minecraft_version: &str,
    _desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ModEntry> {
    let api_key = std::env::var("ATLAS_CURSEFORGE_API_KEY")
        .context("ATLAS_CURSEFORGE_API_KEY is required for CurseForge lookups")?;

    let client = Client::new();
    let class_id = class_id_for(pack_type)?;
    let loader_id = loader_id_for(loader)?;

    let mut serializer = Serializer::new(String::new());
    serializer.append_pair("gameId", &GAME_ID_MINECRAFT.to_string());
    serializer.append_pair("searchFilter", query);
    serializer.append_pair("pageSize", "1");
    serializer.append_pair("classId", &class_id.to_string());
    serializer.append_pair("modLoaderType", &loader_id.to_string());
    serializer.append_pair("gameVersion", minecraft_version);

    let url = format!(
        "https://api.curseforge.com/v1/mods/search?{}",
        serializer.finish()
    );
    let mods = client
        .get(url)
        .header("x-api-key", api_key)
        .send()
        .context("CurseForge search failed")?
        .error_for_status()
        .context("CurseForge search returned an error")?
        .json::<CfResponse<CfMod>>()
        .context("Failed to parse CurseForge search response")?;

    let mod_entry = mods.data.first().context("No CurseForge results found")?;

    let files_url = format!(
        "https://api.curseforge.com/v1/mods/{}/files?gameVersion={}&modLoaderType={}&pageSize=1",
        mod_entry.id, minecraft_version, loader_id
    );
    let files = client
        .get(files_url)
        .header("x-api-key", std::env::var("ATLAS_CURSEFORGE_API_KEY")?)
        .send()
        .context("Failed to load CurseForge files")?
        .error_for_status()
        .context("CurseForge files returned an error")?
        .json::<CfResponse<CfFile>>()
        .context("Failed to parse CurseForge files response")?;

    let file = files.data.first().context("No CurseForge files found")?;
    let download_url = file
        .downloadUrl
        .clone()
        .filter(|value| !value.trim().is_empty())
        .context("CurseForge did not return a downloadable URL.")?;
    let sha1 = file.hashes.iter().find(|hash| hash.algo == 1).map(|h| h.value.clone());

    Ok(ModEntry {
        source: "curseforge".to_string(),
        project_id: mod_entry.id.to_string(),
        version: file.displayName.clone(),
        file_id: Some(file.id.to_string()),
        download_url: Some(download_url),
        hashes: Some(ModHashes {
            sha1,
            sha512: None,
        }),
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
