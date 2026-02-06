use std::cmp::Ordering;
use std::collections::HashSet;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

const MC_VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FABRIC_LOADER_URL_TEMPLATE: &str = "https://meta.fabricmc.net/v2/versions/loader";
const FORGE_MAVEN_METADATA_URL: &str =
    "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const NEOFORGE_MAVEN_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

pub struct VersionCatalog {
    client: Client,
}

impl VersionCatalog {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    pub fn fetch_minecraft_versions(&self) -> Result<Vec<String>> {
        let manifest = self.get_json::<VersionManifest>(MC_VERSION_MANIFEST_URL)?;

        let mut versions = manifest
            .versions
            .into_iter()
            .filter(|version| version.kind == "release")
            .map(|version| version.id)
            .collect::<Vec<_>>();

        sort_versions_desc(&mut versions);
        dedupe_versions(&mut versions);

        if versions.is_empty() {
            bail!("No Minecraft release versions were returned by Mojang");
        }

        Ok(versions)
    }

    pub fn fetch_loader_versions(&self, modloader: &str, mc_version: &str) -> Result<Vec<String>> {
        match modloader {
            "fabric" => self.fetch_fabric_versions(mc_version),
            "forge" => self.fetch_forge_versions(mc_version),
            "neoforge" => self.fetch_neoforge_versions(mc_version),
            _ => bail!("Unsupported modloader: {}", modloader),
        }
    }

    fn fetch_fabric_versions(&self, mc_version: &str) -> Result<Vec<String>> {
        let url = format!("{}/{}", FABRIC_LOADER_URL_TEMPLATE, mc_version);
        let versions = self.get_json::<Vec<FabricLoaderVersion>>(&url)?;

        let mut stable = versions
            .iter()
            .filter(|entry| entry.loader.stable.unwrap_or(true))
            .map(|entry| entry.loader.version.clone())
            .collect::<Vec<_>>();

        if stable.is_empty() {
            stable = versions
                .into_iter()
                .map(|entry| entry.loader.version)
                .collect::<Vec<_>>();
        }

        sort_versions_desc(&mut stable);
        dedupe_versions(&mut stable);

        if stable.is_empty() {
            bail!(
                "No Fabric loader versions found for Minecraft version {}",
                mc_version
            );
        }

        Ok(stable)
    }

    fn fetch_forge_versions(&self, mc_version: &str) -> Result<Vec<String>> {
        let metadata = self.get_text(FORGE_MAVEN_METADATA_URL)?;
        let prefix = format!("{}-", mc_version);

        let mut versions = extract_versions_from_maven_metadata(&metadata)
            .into_iter()
            .filter_map(|entry| entry.strip_prefix(&prefix).map(ToOwned::to_owned))
            .collect::<Vec<_>>();

        sort_versions_desc(&mut versions);
        dedupe_versions(&mut versions);

        if versions.is_empty() {
            bail!(
                "No Forge versions found for Minecraft version {}",
                mc_version
            );
        }

        Ok(versions)
    }

    fn fetch_neoforge_versions(&self, mc_version: &str) -> Result<Vec<String>> {
        let metadata = self.get_text(NEOFORGE_MAVEN_METADATA_URL)?;
        let all_versions = extract_versions_from_maven_metadata(&metadata);
        let candidate_lines = neoforge_candidate_lines(mc_version)?;

        let mut versions = Vec::new();
        for line in candidate_lines {
            let exact_prefix = format!("{}.", line);
            versions = all_versions
                .iter()
                .filter(|version| version.starts_with(&exact_prefix))
                .cloned()
                .collect::<Vec<_>>();

            if versions.is_empty() {
                let fallback_prefix = format!("{}-", line);
                versions = all_versions
                    .iter()
                    .filter(|version| version.starts_with(&fallback_prefix))
                    .cloned()
                    .collect::<Vec<_>>();
            }

            if !versions.is_empty() {
                break;
            }
        }

        sort_versions_desc(&mut versions);
        dedupe_versions(&mut versions);

        if versions.is_empty() {
            bail!(
                "No NeoForge versions found for Minecraft version {}",
                mc_version
            );
        }

        Ok(versions)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T> {
        self.client
            .get(url)
            .header(USER_AGENT, "atlas-cli")
            .send()
            .with_context(|| format!("Failed to fetch {}", url))?
            .error_for_status()
            .with_context(|| format!("Request returned an error for {}", url))?
            .json::<T>()
            .with_context(|| format!("Failed to parse response from {}", url))
    }

    fn get_text(&self, url: &str) -> Result<String> {
        self.client
            .get(url)
            .header(USER_AGENT, "atlas-cli")
            .send()
            .with_context(|| format!("Failed to fetch {}", url))?
            .error_for_status()
            .with_context(|| format!("Request returned an error for {}", url))?
            .text()
            .with_context(|| format!("Failed to parse text response from {}", url))
    }
}

#[derive(Deserialize)]
struct VersionManifest {
    versions: Vec<VersionManifestEntry>,
}

#[derive(Deserialize)]
struct VersionManifestEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    loader: FabricLoaderInfo,
}

#[derive(Deserialize)]
struct FabricLoaderInfo {
    version: String,
    stable: Option<bool>,
}

fn extract_versions_from_maven_metadata(metadata: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = metadata;

    loop {
        let Some(start_index) = rest.find("<version>") else {
            break;
        };
        let after_start = &rest[start_index + "<version>".len()..];
        let Some(end_index) = after_start.find("</version>") else {
            break;
        };

        let version = after_start[..end_index].trim();
        if !version.is_empty() {
            versions.push(version.to_string());
        }

        rest = &after_start[end_index + "</version>".len()..];
    }

    versions
}

fn neoforge_candidate_lines(mc_version: &str) -> Result<Vec<String>> {
    let parts = mc_version.split('.').collect::<Vec<_>>();
    if parts.len() < 3 || parts[0] != "1" {
        bail!(
            "Unsupported Minecraft version format for NeoForge: {}",
            mc_version
        );
    }

    let major = parts[1]
        .parse::<u64>()
        .with_context(|| format!("Invalid Minecraft version: {}", mc_version))?;
    let patch = parts[2]
        .parse::<u64>()
        .with_context(|| format!("Invalid Minecraft version: {}", mc_version))?;

    // NeoForge 1.20.1 builds are published on the 20.2 line.
    if major == 20 && patch == 1 {
        return Ok(vec!["20.2".to_string(), "20.1".to_string()]);
    }

    Ok(vec![format!("{}.{}", major, patch)])
}

fn dedupe_versions(versions: &mut Vec<String>) {
    let mut seen = HashSet::new();
    versions.retain(|version| seen.insert(version.clone()));
}

fn sort_versions_desc(versions: &mut [String]) {
    versions.sort_by(|left, right| compare_version_like(right, left));
}

fn compare_version_like(left: &str, right: &str) -> Ordering {
    let left_tokens = tokenize_version(left);
    let right_tokens = tokenize_version(right);
    let max = left_tokens.len().max(right_tokens.len());

    for index in 0..max {
        let left_token = left_tokens.get(index);
        let right_token = right_tokens.get(index);
        match (left_token, right_token) {
            (Some(VersionToken::Number(left_num)), Some(VersionToken::Number(right_num))) => {
                let order = left_num.cmp(right_num);
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(VersionToken::Text(left_text)), Some(VersionToken::Text(right_text))) => {
                let order = left_text.cmp(right_text);
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(VersionToken::Number(_)), Some(VersionToken::Text(_))) => {
                return Ordering::Greater;
            }
            (Some(VersionToken::Text(_)), Some(VersionToken::Number(_))) => {
                return Ordering::Less;
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => break,
        }
    }

    left.cmp(right)
}

#[derive(Debug)]
enum VersionToken {
    Number(u64),
    Text(String),
}

fn tokenize_version(value: &str) -> Vec<VersionToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut current_is_number: Option<bool> = None;

    for ch in value.chars() {
        if ch.is_ascii_digit() {
            if current_is_number == Some(true) {
                current.push(ch);
            } else {
                push_token(&mut tokens, &mut current, current_is_number);
                current.push(ch);
                current_is_number = Some(true);
            }
            continue;
        }

        if ch.is_ascii_alphabetic() {
            let lower = ch.to_ascii_lowercase();
            if current_is_number == Some(false) {
                current.push(lower);
            } else {
                push_token(&mut tokens, &mut current, current_is_number);
                current.push(lower);
                current_is_number = Some(false);
            }
            continue;
        }

        push_token(&mut tokens, &mut current, current_is_number);
        current_is_number = None;
    }

    push_token(&mut tokens, &mut current, current_is_number);
    tokens
}

fn push_token(
    tokens: &mut Vec<VersionToken>,
    current: &mut String,
    current_is_number: Option<bool>,
) {
    if current.is_empty() {
        return;
    }

    if current_is_number == Some(true) {
        let value = current.parse::<u64>().unwrap_or(0);
        tokens.push(VersionToken::Number(value));
    } else {
        tokens.push(VersionToken::Text(current.clone()));
    }

    current.clear();
}

#[cfg(test)]
mod tests {
    use super::neoforge_candidate_lines;

    #[test]
    fn neoforge_line_supports_1201_alias() {
        let lines = neoforge_candidate_lines("1.20.1").expect("valid mc version");
        assert_eq!(lines, vec!["20.2".to_string(), "20.1".to_string()]);
    }

    #[test]
    fn neoforge_line_uses_patch_for_standard_versions() {
        let lines = neoforge_candidate_lines("1.21.1").expect("valid mc version");
        assert_eq!(lines, vec!["21.1".to_string()]);
    }
}
