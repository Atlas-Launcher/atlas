use crate::launcher::download::download_raw;
use crate::launcher::error::LauncherError;
use crate::launcher::manifest::VersionData;
use crate::net::http::{fetch_text, shared_client, HttpError};
use crate::paths::ensure_dir;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const NEOFORGE_METADATA_URL: &str =
    "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

pub async fn fetch_loader_versions(client: &Client) -> Result<Vec<String>, HttpError> {
    let xml = fetch_text(client, NEOFORGE_METADATA_URL).await?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut versions = Vec::new();
    let mut in_versions = false;
    let mut in_version = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => match event.name().as_ref() {
                b"versions" => in_versions = true,
                b"version" if in_versions => in_version = true,
                _ => {}
            },
            Ok(Event::End(event)) => match event.name().as_ref() {
                b"versions" => in_versions = false,
                b"version" => in_version = false,
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if in_versions && in_version {
                    let value = text.decode().map_err(|err| HttpError::ParseMessage {
                        message: err.to_string(),
                        body: xml.clone(),
                    })?;
                    let trimmed = value.trim();
                    if !trimmed.is_empty() {
                        versions.push(trimmed.to_string());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(HttpError::ParseMessage {
                    message: err.to_string(),
                    body: xml.clone(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    versions.reverse();
    Ok(versions)
}

pub async fn ensure_profile(
    game_dir: &Path,
    loader_version: &str,
) -> Result<VersionData, LauncherError> {
    let loader_version = loader_version.trim();
    if loader_version.is_empty() {
        return Err("NeoForge loader version is required.".to_string().into());
    }
    let version_id = format!("neoforge-{loader_version}");
    let version_dir = game_dir.join("versions").join(&version_id);
    let version_json_path = version_dir.join(format!("{version_id}.json"));

    if version_json_path.exists() {
        return read_profile(&version_json_path);
    }

    ensure_dir(&version_dir)?;
    let installer_path = version_dir.join(format!("{version_id}-installer.jar"));
    let installer_url = installer_url(loader_version);
    let client = shared_client().clone();

    if !installer_path.exists() || installer_path.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
        download_raw(&client, &installer_url, &installer_path, None, true).await?;
    }

    let version_bytes = extract_version_json(&installer_path)?;
    std::fs::write(&version_json_path, &version_bytes)
        .map_err(|err| format!("Failed to write NeoForge profile: {err}"))?;
    read_profile(&version_json_path)
}

fn installer_url(loader_version: &str) -> String {
    format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader_version}/neoforge-{loader_version}-installer.jar")
}

fn extract_version_json(installer_path: &Path) -> Result<Vec<u8>, LauncherError> {
    let file =
        std::fs::File::open(installer_path).map_err(|err| format!("Open installer: {err}"))?;
    let mut archive = ZipArchive::new(file).map_err(|err| format!("Read installer jar: {err}"))?;

    if let Ok(mut entry) = archive.by_name("version.json") {
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Read installer version.json: {err}"))?;
        return Ok(buffer);
    }

    let mut fallback: Option<(PathBuf, Vec<u8>)> = None;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| format!("Read installer entry: {err}"))?;
        let name = entry.name().to_string();
        if !name.ends_with("version.json") {
            continue;
        }
        let mut buffer = Vec::new();
        entry
            .read_to_end(&mut buffer)
            .map_err(|err| format!("Read installer version.json: {err}"))?;
        let path = PathBuf::from(name);
        fallback = Some((path, buffer));
        break;
    }

    if let Some((_, buffer)) = fallback {
        return Ok(buffer);
    }

    Err("NeoForge installer missing version.json".to_string().into())
}

fn read_profile(path: &Path) -> Result<VersionData, LauncherError> {
    let bytes =
        std::fs::read(path).map_err(|err| format!("Failed to read NeoForge profile: {err}"))?;
    serde_json::from_slice::<VersionData>(&bytes)
        .map_err(|err| format!("Failed to parse NeoForge profile: {err}").into())
}
