use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{ResolvedDependency, ResolvedMod, SearchCandidate};
use protocol::config::mods::{ModDownload, ModEntry, ModHashes, ModMetadata, ModSide};

const GAME_ID_MINECRAFT: i32 = 432;
const DEPENDENCY_REQUIRED: i32 = 3;

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
    slug: Option<String>,
    summary: Option<String>,
}

#[derive(Deserialize)]
struct CfFile {
    id: i64,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "downloadUrl")]
    download_url: Option<String>,
    #[serde(rename = "gameVersions", default)]
    game_versions: Vec<String>,
    #[serde(default)]
    dependencies: Vec<CfDependency>,
    hashes: Vec<CfHash>,
}

#[derive(Deserialize)]
struct CfDependency {
    #[serde(rename = "modId")]
    mod_id: i64,
    #[serde(rename = "relationType")]
    relation_type: i32,
}

#[derive(Deserialize)]
struct CfHash {
    algo: i32,
    value: String,
}

pub async fn search(
    client: &reqwest::Client,
    proxy_base_url: &str,
    access_token: &str,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let class_id = class_id_for(pack_type)?;
    let loader_id = loader_id_for(loader)?;
    let base = format!("{}/api/v1/curseforge", proxy_base_url.trim_end_matches('/'));

    let mut search_url = reqwest::Url::parse(&format!("{base}/mods/search"))
        .context("Failed to build CurseForge proxy search URL")?;
    {
        let mut pairs = search_url.query_pairs_mut();
        pairs.append_pair("gameId", &GAME_ID_MINECRAFT.to_string());
        pairs.append_pair("searchFilter", query);
        pairs.append_pair("gameVersion", minecraft_version);
        pairs.append_pair("index", &offset.to_string());
        pairs.append_pair("pageSize", &limit.clamp(1, 50).to_string());
        if let Some(class_id) = class_id {
            pairs.append_pair("classId", &class_id.to_string());
        }
        if include_loader_filter(pack_type) {
            pairs.append_pair("modLoaderType", &loader_id.to_string());
        }
    }

    let response = client
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

    Ok(response
        .data
        .into_iter()
        .map(|item| {
            let slug = item.slug.unwrap_or_else(|| item.id.to_string());
            SearchCandidate {
                project_id: item.id.to_string(),
                slug: slug.clone(),
                title: item.name,
                description: item.summary,
                project_url: Some(curseforge_project_url(&slug)),
            }
        })
        .collect())
}

pub async fn resolve_by_project_id(
    client: &reqwest::Client,
    proxy_base_url: &str,
    access_token: &str,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let mod_id = project_id
        .parse::<i64>()
        .context("CurseForge project id must be numeric")?;
    let class_id = class_id_for(pack_type)?;
    let loader_id = loader_id_for(loader)?;
    let base = format!("{}/api/v1/curseforge", proxy_base_url.trim_end_matches('/'));

    let mut files_url = reqwest::Url::parse(&format!("{base}/mods/{mod_id}/files"))
        .context("Failed to build CurseForge proxy files URL")?;
    {
        let mut pairs = files_url.query_pairs_mut();
        pairs.append_pair("gameVersion", minecraft_version);
        pairs.append_pair("pageSize", "50");
        if let Some(class_id) = class_id {
            pairs.append_pair("classId", &class_id.to_string());
        }
        if include_loader_filter(pack_type) {
            pairs.append_pair("modLoaderType", &loader_id.to_string());
        }
    }

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

    let file = select_compatible_file(&files.data, minecraft_version, desired_version)
        .context("No compatible CurseForge files found for this Minecraft version/loader")?;

    let download_url = if let Some(url) = file.download_url.clone().filter(|v| !v.trim().is_empty())
    {
        url
    } else {
        let url = format!("{base}/mods/{mod_id}/files/{}/download-url", file.id);
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

    let dependencies = file
        .dependencies
        .iter()
        .filter(|dependency| dependency.relation_type == DEPENDENCY_REQUIRED)
        .map(|dependency| dependency.mod_id.to_string())
        .filter(|dependency_id| dependency_id != project_id)
        .map(|dependency_project_id| ResolvedDependency {
            project_id: dependency_project_id,
            desired_version: None,
        })
        .collect::<Vec<_>>();

    Ok(ResolvedMod {
        entry: ModEntry {
            metadata: ModMetadata {
                name: project_id.to_string(),
                side: side_for_pack_type(pack_type),
                project_url: Some(curseforge_project_url(project_id)),
                disabled_client_oses: Vec::new(),
            },
            download: ModDownload {
                source: "curseforge".to_string(),
                project_id: project_id.to_string(),
                version: file.display_name.clone(),
                file_id: Some(file.id.to_string()),
                url: Some(download_url),
                hashes: Some(ModHashes {
                    sha1,
                    sha256: None,
                    sha512: None,
                }),
            },
        },
        dependencies,
    })
}

fn select_compatible_file<'a>(
    files: &'a [CfFile],
    minecraft_version: &str,
    desired_version: Option<&str>,
) -> Option<&'a CfFile> {
    let compatible = files
        .iter()
        .filter(|file| {
            file.game_versions.is_empty()
                || file
                    .game_versions
                    .iter()
                    .any(|value| value == minecraft_version)
        })
        .collect::<Vec<_>>();

    if let Some(desired) = desired_version {
        return compatible
            .into_iter()
            .find(|file| file.display_name == desired || file.file_name == desired);
    }

    compatible.into_iter().next()
}

fn loader_id_for(loader: &str) -> Result<i32> {
    match loader.to_lowercase().as_str() {
        "fabric" => Ok(4),
        "forge" => Ok(1),
        "neo" | "neoforge" => Ok(6),
        other => bail!("Unsupported loader for CurseForge: {}", other),
    }
}

fn class_id_for(pack_type: &str) -> Result<Option<i32>> {
    match pack_type {
        "mod" => Ok(Some(6)),
        "shader" => Ok(Some(6552)),
        "resourcepack" => Ok(Some(12)),
        "other" => Ok(None),
        other => bail!("Unsupported pack type for CurseForge: {}", other),
    }
}

fn include_loader_filter(pack_type: &str) -> bool {
    matches!(pack_type, "mod")
}

fn side_for_pack_type(pack_type: &str) -> ModSide {
    match pack_type {
        "shader" | "resourcepack" => ModSide::Client,
        _ => ModSide::Both,
    }
}

fn curseforge_project_url(slug_or_id: &str) -> String {
    format!(
        "https://www.curseforge.com/minecraft/mc-mods/{}",
        slug_or_id.trim()
    )
}
