use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{ResolvedDependency, ResolvedMod, SearchCandidate};
use protocol::config::mods::{ModDownload, ModEntry, ModHashes, ModMetadata, ModSide};

#[derive(Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
}

#[derive(Deserialize)]
struct SearchHit {
    project_id: String,
    slug: Option<String>,
    title: Option<String>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct ProjectInfo {
    id: String,
    slug: Option<String>,
    title: Option<String>,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    client_side: Option<String>,
    #[serde(default)]
    server_side: Option<String>,
}

#[derive(Deserialize)]
struct VersionInfo {
    id: String,
    name: String,
    version_number: String,
    #[serde(default)]
    side: Option<String>,
    #[serde(default)]
    client_side: Option<String>,
    #[serde(default)]
    server_side: Option<String>,
    files: Vec<ModFile>,
    #[serde(default)]
    dependencies: Vec<ModDependency>,
}

#[derive(Deserialize)]
struct ModFile {
    url: String,
    hashes: ModHashes,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct ModDependency {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    version_id: Option<String>,
    #[serde(default)]
    dependency_type: String,
}

#[derive(Deserialize)]
struct VersionLookup {
    project_id: String,
}

pub async fn search(
    client: &reqwest::Client,
    query: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SearchCandidate>> {
    let facets = build_search_facets(pack_type, loader, minecraft_version);
    let offset_str = offset.to_string();
    let limit_str = limit.clamp(1, 50).to_string();
    let search_url = reqwest::Url::parse_with_params(
        "https://api.modrinth.com/v2/search",
        [
            ("query", query),
            ("offset", offset_str.as_str()),
            ("limit", limit_str.as_str()),
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

    Ok(search
        .hits
        .into_iter()
        .map(|hit| {
            let slug = hit.slug.unwrap_or_else(|| hit.project_id.clone());
            SearchCandidate {
                project_id: hit.project_id.clone(),
                slug: slug.clone(),
                title: hit.title.unwrap_or_else(|| hit.project_id.clone()),
                description: hit.description,
                project_url: Some(format!("https://modrinth.com/mod/{}", slug)),
            }
        })
        .collect())
}

pub async fn resolve_by_project_id(
    client: &reqwest::Client,
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    desired_version: Option<&str>,
    pack_type: &str,
) -> Result<ResolvedMod> {
    let project_url = format!("https://api.modrinth.com/v2/project/{}", project_id);
    let project = client
        .get(project_url)
        .send()
        .await
        .context("Failed to load Modrinth project")?
        .error_for_status()
        .context("Modrinth project returned an error")?
        .json::<ProjectInfo>()
        .await
        .context("Failed to parse Modrinth project response")?;

    let version_url = build_version_url(project_id, loader, minecraft_version, pack_type);
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
            .find(|item| {
                item.id == desired || item.version_number == desired || item.name == desired
            })
            .context("Requested Modrinth version not found for this Minecraft version/loader")?
    } else {
        versions
            .first()
            .context("No compatible Modrinth versions found for this Minecraft version/loader")?
    };

    let file = version
        .files
        .iter()
        .find(|candidate| candidate.primary)
        .or_else(|| version.files.first())
        .context("No Modrinth files found for selected version")?;

    if file.url.trim().is_empty() {
        bail!("Modrinth did not return a downloadable URL.");
    }

    let mut dependencies: Vec<ResolvedDependency> = Vec::new();
    for dependency in &version.dependencies {
        if !should_include_dependency_type(&dependency.dependency_type) {
            continue;
        }

        let Some(dependency_project_id) = resolve_dependency_project_id(client, dependency).await?
        else {
            continue;
        };
        if dependency_project_id == project.id {
            continue;
        }

        if let Some(existing) = dependencies
            .iter_mut()
            .find(|existing| existing.project_id == dependency_project_id)
        {
            if existing.desired_version.is_none() {
                existing.desired_version = dependency.version_id.clone();
            }
            continue;
        }

        dependencies.push(ResolvedDependency {
            project_id: dependency_project_id,
            desired_version: dependency.version_id.clone(),
        });
    }

    let project_url = project
        .slug
        .as_ref()
        .map(|slug| format!("https://modrinth.com/mod/{}", slug))
        .unwrap_or_else(|| format!("https://modrinth.com/mod/{}", project.id));

    let side = map_side(
        pack_type,
        version.side.as_deref().or(project.side.as_deref()),
        version
            .client_side
            .as_deref()
            .or(project.client_side.as_deref()),
        version
            .server_side
            .as_deref()
            .or(project.server_side.as_deref()),
    );

    Ok(ResolvedMod {
        entry: ModEntry {
            metadata: ModMetadata {
                name: project
                    .title
                    .clone()
                    .or(project.slug.clone())
                    .unwrap_or_else(|| project.id.clone()),
                side,
                project_url: Some(project_url),
                disabled_client_oses: Vec::new(),
            },
            download: ModDownload {
                source: "modrinth".to_string(),
                project_id: project.id.clone(),
                version: version
                    .id
                    .trim()
                    .to_string()
                    .if_empty_then(|| version.version_number.clone()),
                file_id: None,
                url: Some(file.url.clone()),
                hashes: Some(ModHashes {
                    sha1: None,
                    sha512: file.hashes.sha512.clone(),
                }),
            },
        },
        dependencies,
    })
}

async fn resolve_dependency_project_id(
    client: &reqwest::Client,
    dependency: &ModDependency,
) -> Result<Option<String>> {
    if let Some(project_id) = dependency
        .project_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(project_id.to_string()));
    }

    let Some(version_id) = dependency
        .version_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let version = client
        .get(format!("https://api.modrinth.com/v2/version/{version_id}"))
        .send()
        .await
        .context("Failed to load Modrinth dependency version")?
        .error_for_status()
        .context("Modrinth dependency version returned an error")?
        .json::<VersionLookup>()
        .await
        .context("Failed to parse Modrinth dependency version response")?;

    let project_id = version.project_id.trim();
    if project_id.is_empty() {
        return Ok(None);
    }

    Ok(Some(project_id.to_string()))
}

fn map_side(
    pack_type: &str,
    side_value: Option<&str>,
    client: Option<&str>,
    server: Option<&str>,
) -> ModSide {
    match pack_type {
        "shader" | "resourcepack" => return ModSide::Client,
        _ => {}
    }

    if let Some(side) = side_value
        .and_then(map_explicit_side_value)
        .or_else(|| client.and_then(map_explicit_side_value))
        .or_else(|| server.and_then(map_explicit_side_value))
    {
        return side;
    }

    let client_supported = !matches!(
        client.map(str::to_ascii_lowercase).as_deref(),
        Some("unsupported")
    );
    let server_supported = !matches!(
        server.map(str::to_ascii_lowercase).as_deref(),
        Some("unsupported")
    );
    match (client_supported, server_supported) {
        (true, false) => ModSide::Client,
        (false, true) => ModSide::Server,
        _ => ModSide::Both,
    }
}

fn map_explicit_side_value(value: &str) -> Option<ModSide> {
    match value.trim().to_ascii_lowercase().as_str() {
        "client_only" => Some(ModSide::Client),
        "server_only" => Some(ModSide::Both),
        "dedicated_server_only" => Some(ModSide::Server),
        "client_and_server" => Some(ModSide::Both),
        "server_only_client_optional" => Some(ModSide::Both),
        "client_only_server_optional" => Some(ModSide::Both),
        "client_or_server_prefers_both" => Some(ModSide::Both),
        "client_or_server" => Some(ModSide::Both),
        "singleplayer_only" => Some(ModSide::Client),
        _ => None,
    }
}

fn should_include_dependency_type(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    matches!(normalized.as_str(), "required" | "optional")
}

fn build_search_facets(pack_type: &str, loader: &str, minecraft_version: &str) -> String {
    match pack_type {
        "mod" => format!(
            "[[\"project_type:mod\"],[\"categories:{}\"],[\"versions:{}\"]]",
            loader, minecraft_version
        ),
        "shader" => format!(
            "[[\"project_type:shader\"],[\"versions:{}\"]]",
            minecraft_version
        ),
        "resourcepack" => format!(
            "[[\"project_type:resourcepack\"],[\"versions:{}\"]]",
            minecraft_version
        ),
        "other" => format!("[[\"versions:{}\"]]", minecraft_version),
        _ => format!(
            "[[\"project_type:{}\"],[\"versions:{}\"]]",
            pack_type, minecraft_version
        ),
    }
}

fn build_version_url(
    project_id: &str,
    loader: &str,
    minecraft_version: &str,
    pack_type: &str,
) -> String {
    match pack_type {
        "mod" => format!(
            "https://api.modrinth.com/v2/project/{}/version?loaders=[\"{}\"]&game_versions=[\"{}\"]",
            project_id, loader, minecraft_version
        ),
        _ => format!(
            "https://api.modrinth.com/v2/project/{}/version?game_versions=[\"{}\"]",
            project_id, minecraft_version
        ),
    }
}

trait IfEmptyThen {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String;
}

impl IfEmptyThen for String {
    fn if_empty_then<F: FnOnce() -> String>(self, fallback: F) -> String {
        if self.trim().is_empty() {
            fallback()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ModSide, map_side, should_include_dependency_type};

    #[test]
    fn maps_explicit_modrinth_side_values() {
        assert_eq!(
            map_side("mod", Some("client_only"), None, None),
            ModSide::Client
        );
        assert_eq!(
            map_side("mod", Some("server_only"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("dedicated_server_only"), None, None),
            ModSide::Server
        );
        assert_eq!(
            map_side("mod", Some("client_and_server"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("server_only_client_optional"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("client_only_server_optional"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("client_or_server_prefers_both"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("client_or_server"), None, None),
            ModSide::Both
        );
        assert_eq!(
            map_side("mod", Some("singleplayer_only"), None, None),
            ModSide::Client
        );
    }

    #[test]
    fn includes_required_and_optional_dependencies() {
        assert!(should_include_dependency_type("required"));
        assert!(should_include_dependency_type("optional"));
        assert!(!should_include_dependency_type("embedded"));
        assert!(!should_include_dependency_type("incompatible"));
    }
}
