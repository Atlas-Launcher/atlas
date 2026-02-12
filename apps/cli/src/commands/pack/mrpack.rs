use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mod_resolver::pointer::PointerKind as ResolverPointerKind;
use mod_resolver::{Provider, SearchCandidate};
use protocol::config::mods::{ModDownload, ModEntry, ModHashes, ModMetadata, ModSide};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{AssetKind, BuildArgs, ImportArgs};

pub(super) fn import(args: ImportArgs) -> Result<()> {
    let root = args
        .input
        .canonicalize()
        .context("Failed to resolve input path")?;
    let mrpack_path = args
        .file
        .canonicalize()
        .with_context(|| format!("Failed to resolve mrpack path: {}", args.file.display()))?;
    if !mrpack_path.is_file() {
        bail!("Expected a file at {}", mrpack_path.display());
    }

    let config = crate::config::load_atlas_config(&root)?;
    let loader = config.versions.modloader;
    let minecraft_version = config.versions.mc;

    let mut archive = open_archive(&mrpack_path)?;
    let index = load_mrpack_index(&mut archive)?;
    let mut existing = super::load_existing_mod_keys(&root)?;

    let mut added_from_index = 0usize;
    let mut skipped_existing_index = 0usize;
    let mut imported_override_text = 0usize;
    let mut skipped_binary_overrides = 0usize;
    let mut added_override_mods = 0usize;
    let mut unresolved_override_mods = Vec::new();

    for file in &index.files {
        let Some(kind) = asset_kind_for_path(&file.path) else {
            continue;
        };

        let Some(entry) = build_entry_from_mrpack_file(file, kind) else {
            continue;
        };
        let key = super::mod_key(&entry.download.source, &entry.download.project_id);
        if existing.insert(key) {
            if kind == AssetKind::Mod {
                crate::io::write_mod_entry(&root, &entry)?;
            } else {
                crate::io::write_resource_entry(&root, &entry, kind.resource_pointer_directory())?;
            }
            added_from_index += 1;
        } else {
            skipped_existing_index += 1;
        }
    }

    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .with_context(|| format!("Failed to read archive entry #{}", index))?;
        if file.is_dir() {
            continue;
        }

        let entry_name = file.name().replace('\\', "/");
        let Some(rel_path) = strip_overrides_prefix(&entry_name) else {
            continue;
        };
        if rel_path.trim().is_empty() {
            continue;
        }

        if is_override_mod_path(&rel_path) {
            match resolve_override_mod_entry(&rel_path, &loader, &minecraft_version) {
                Ok(Some(entry)) => {
                    let key = super::mod_key(&entry.download.source, &entry.download.project_id);
                    if existing.insert(key) {
                        crate::io::write_mod_entry(&root, &entry)?;
                        added_override_mods += 1;
                        println!(
                            "Added override mod {}",
                            super::mod_reference_for_entry(&entry)
                        );
                    } else {
                        skipped_existing_index += 1;
                    }
                }
                Ok(None) => {
                    eprintln!("can't find asset: {}", rel_path);
                    unresolved_override_mods.push(rel_path);
                }
                Err(error) => {
                    eprintln!("can't find asset: {} ({})", rel_path, error);
                    unresolved_override_mods.push(rel_path);
                }
            }
            continue;
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .with_context(|| format!("Failed to read {}", entry_name))?;
        if std::str::from_utf8(&bytes).is_ok() {
            write_override_text_file(&root, &rel_path, &bytes)?;
            imported_override_text += 1;
        } else {
            skipped_binary_overrides += 1;
        }
    }

    println!(
        "Imported {} pointer entries from mrpack index.",
        added_from_index
    );
    if skipped_existing_index > 0 {
        println!(
            "Skipped {} existing pointer entries.",
            skipped_existing_index
        );
    }
    println!("Imported {} override text file(s).", imported_override_text);
    if added_override_mods > 0 {
        println!("Added {} override mod pointer(s).", added_override_mods);
    }
    if skipped_binary_overrides > 0 {
        println!(
            "Skipped {} binary override file(s) (only UTF-8 text files are imported).",
            skipped_binary_overrides
        );
    }
    if !unresolved_override_mods.is_empty() {
        println!(
            "Could not resolve {} override mod asset(s). The import still completed.",
            unresolved_override_mods.len()
        );
        for rel in &unresolved_override_mods {
            println!("  - can't find asset: {}", rel);
        }
    }

    Ok(())
}

pub(super) fn build(args: &BuildArgs, root: &Path) -> Result<()> {
    let atlas = crate::config::load_atlas_config(root)?;
    let output = normalize_mrpack_output(&args.output);

    let mut files = Vec::new();
    let mut override_files = Vec::new();

    for pointer_path in super::pointer_paths(root)? {
        let rel_path = pointer_path
            .strip_prefix(root)
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| pointer_path.to_string_lossy().replace('\\', "/"));
        let contents = crate::io::read_to_string(&pointer_path)?;
        let entry = protocol::config::mods::parse_mod_toml(&contents)
            .map_err(|_| anyhow::anyhow!("Invalid pointer file: {}", pointer_path.display()))?;

        let Some(download_url) = entry
            .download
            .url
            .clone()
            .filter(|value| !value.trim().is_empty())
        else {
            eprintln!("Skipping {} because it has no download URL.", rel_path);
            continue;
        };

        let resolver_kind = if rel_path.ends_with(".mod.toml") {
            ResolverPointerKind::Mod
        } else {
            ResolverPointerKind::Resource
        };
        let asset_rel_path = mod_resolver::pointer::destination_relative_path(
            &rel_path,
            resolver_kind,
            &download_url,
        );

        let is_curseforge_mod = resolver_kind == ResolverPointerKind::Mod
            && entry
                .download
                .source
                .trim()
                .eq_ignore_ascii_case("curseforge");

        if is_curseforge_mod {
            match download_url_bytes(&download_url) {
                Ok(bytes) => {
                    override_files.push((asset_rel_path, bytes));
                }
                Err(error) => {
                    eprintln!(
                        "can't find asset: {} ({})",
                        entry.download.project_id, error
                    );
                }
            }
            continue;
        }

        let Some(sha512) = ensure_sha512(&entry, &download_url)? else {
            eprintln!("Skipping {} because no sha512 could be derived.", rel_path);
            continue;
        };

        files.push(MrpackBuildFile {
            path: asset_rel_path,
            hashes: {
                let mut hashes = BTreeMap::new();
                hashes.insert("sha512".to_string(), sha512);
                hashes
            },
            env: Some(MrpackBuildEnv {
                client: side_env_value(entry.metadata.side, true),
                server: side_env_value(entry.metadata.side, false),
            }),
            downloads: vec![download_url],
            file_size: None,
        });
    }

    for (rel_path, bytes) in collect_override_text_files(root)? {
        override_files.push((rel_path, bytes));
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    override_files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut dependencies = BTreeMap::new();
    dependencies.insert("minecraft".to_string(), atlas.versions.mc.clone());
    let loader_key = loader_dependency_key(&atlas.versions.modloader)?;
    dependencies.insert(
        loader_key.to_string(),
        atlas.versions.modloader_version.clone(),
    );

    let index = MrpackBuildIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: args
            .version
            .clone()
            .or(atlas.metadata.version.clone())
            .unwrap_or_else(|| "1.0.0".to_string()),
        name: atlas.metadata.name.clone(),
        summary: atlas.metadata.description.clone(),
        files,
        dependencies,
        hash_format: "sha512".to_string(),
    };

    let index_json =
        serde_json::to_vec_pretty(&index).context("Failed to serialize mrpack index")?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let output_file = fs::File::create(&output)
        .with_context(|| format!("Failed to create {}", output.display()))?;
    let mut zip = ZipWriter::new(output_file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    zip.start_file("modrinth.index.json", options)
        .context("Failed to write modrinth.index.json")?;
    zip.write_all(&index_json)
        .context("Failed to write modrinth.index.json")?;

    for (rel_path, bytes) in override_files {
        let archive_path = format!("overrides/{}", rel_path.trim_start_matches('/'));
        zip.start_file(&archive_path, options)
            .with_context(|| format!("Failed to write {}", archive_path))?;
        zip.write_all(&bytes)
            .with_context(|| format!("Failed to write {}", archive_path))?;
    }

    zip.finish().context("Failed to finalize mrpack")?;
    println!("Wrote {}", output.display());
    Ok(())
}

fn normalize_mrpack_output(output: &Path) -> PathBuf {
    if output == Path::new("dist/atlas-pack.atlas") {
        return PathBuf::from("dist/atlas-pack.mrpack");
    }
    output.to_path_buf()
}

fn collect_override_text_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .context("Failed to compute repo relative path")?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");

        if super::is_excluded_path(&rel_str)
            || rel_str == "atlas.toml"
            || rel_str.ends_with(".mod.toml")
            || rel_str.ends_with(".res.toml")
        {
            continue;
        }

        let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
        if std::str::from_utf8(&bytes).is_ok() {
            files.push((rel_str, bytes));
        }
    }
    Ok(files)
}

fn loader_dependency_key(loader: &str) -> Result<&'static str> {
    match loader.trim().to_ascii_lowercase().as_str() {
        "fabric" => Ok("fabric-loader"),
        "forge" => Ok("forge"),
        "neo" | "neoforge" => Ok("neoforge"),
        other => bail!("Unsupported loader '{}' for mrpack export", other),
    }
}

fn side_env_value(side: ModSide, client: bool) -> String {
    match (side, client) {
        (ModSide::Both, _) => "required".to_string(),
        (ModSide::Client, true) => "required".to_string(),
        (ModSide::Client, false) => "unsupported".to_string(),
        (ModSide::Server, true) => "unsupported".to_string(),
        (ModSide::Server, false) => "required".to_string(),
    }
}

fn ensure_sha512(entry: &ModEntry, download_url: &str) -> Result<Option<String>> {
    if let Some(hash) = entry
        .download
        .hashes
        .as_ref()
        .and_then(|hashes| hashes.sha512.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(hash));
    }

    let bytes = download_url_bytes(download_url)?;
    let mut hasher = Sha512::new();
    hasher.update(bytes);
    Ok(Some(hex::encode(hasher.finalize())))
}

fn download_url_bytes(url: &str) -> Result<Vec<u8>> {
    reqwest::blocking::Client::new()
        .get(url)
        .send()
        .with_context(|| format!("Failed to download {}", url))?
        .error_for_status()
        .with_context(|| format!("Download failed for {}", url))?
        .bytes()
        .with_context(|| format!("Failed to read bytes for {}", url))
        .map(|value| value.to_vec())
}

fn open_archive(path: &Path) -> Result<ZipArchive<fs::File>> {
    let file =
        fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    ZipArchive::new(file)
        .with_context(|| format!("{} is not a valid .mrpack archive", path.display()))
}

fn load_mrpack_index<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Result<MrpackIndex> {
    let mut index_file = archive
        .by_name("modrinth.index.json")
        .context("modrinth.index.json not found in mrpack")?;
    let mut index_json = String::new();
    index_file
        .read_to_string(&mut index_json)
        .context("Failed to read modrinth.index.json")?;
    serde_json::from_str::<MrpackIndex>(&index_json).context("Invalid modrinth.index.json")
}

fn asset_kind_for_path(path: &str) -> Option<AssetKind> {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    if normalized.starts_with("mods/") {
        return Some(AssetKind::Mod);
    }
    if normalized.starts_with("resourcepacks/") {
        return Some(AssetKind::Resourcepack);
    }
    if normalized.starts_with("shaderpacks/") {
        return Some(AssetKind::Shader);
    }
    None
}

fn build_entry_from_mrpack_file(file: &MrpackFile, kind: AssetKind) -> Option<ModEntry> {
    let download_url = file
        .downloads
        .iter()
        .find(|value| !value.trim().is_empty())
        .cloned()?;

    let file_name = Path::new(&file.path)
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "asset".to_string());
    let project_id_fallback = slugify_mod_name(file_name.trim_end_matches(".jar"));

    let parsed = parse_mrpack_download_url(&download_url);
    let source = parsed
        .as_ref()
        .map(|value| value.source.as_str())
        .unwrap_or("mrpack")
        .to_string();
    let project_id = parsed
        .as_ref()
        .and_then(|value| value.project_id.clone())
        .unwrap_or_else(|| {
            if project_id_fallback.is_empty() {
                "asset".to_string()
            } else {
                project_id_fallback.clone()
            }
        });
    let version = parsed
        .as_ref()
        .and_then(|value| value.version.clone())
        .unwrap_or_else(|| file_name.clone());

    Some(ModEntry {
        metadata: ModMetadata {
            name: file_name.clone(),
            side: map_mrpack_side(kind, file.env.as_ref()),
            project_url: project_url_for_source(&source, &project_id),
            disabled_client_oses: Vec::new(),
        },
        download: ModDownload {
            source,
            project_id,
            version,
            file_id: parsed.as_ref().and_then(|value| value.file_id.clone()),
            url: Some(download_url),
            hashes: Some(ModHashes {
                sha1: get_hash(&file.hashes, "sha1"),
                sha256: get_hash(&file.hashes, "sha256"),
                sha512: get_hash(&file.hashes, "sha512"),
            }),
        },
    })
}

fn get_hash(hashes: &HashMap<String, String>, key: &str) -> Option<String> {
    hashes.iter().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case(key) {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn map_mrpack_side(kind: AssetKind, env: Option<&MrpackEnv>) -> ModSide {
    if kind != AssetKind::Mod {
        return ModSide::Client;
    }
    let Some(env) = env else {
        return ModSide::Both;
    };

    let client_supported = side_is_supported(env.client.as_deref());
    let server_supported = side_is_supported(env.server.as_deref());
    match (client_supported, server_supported) {
        (true, true) => ModSide::Both,
        (true, false) => ModSide::Client,
        (false, true) => ModSide::Server,
        (false, false) => ModSide::Both,
    }
}

fn side_is_supported(value: Option<&str>) -> bool {
    !matches!(
        value.map(|raw| raw.trim().to_ascii_lowercase()),
        Some(ref v) if v == "unsupported"
    )
}

fn strip_overrides_prefix(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    for prefix in ["overrides/", "client-overrides/", "server-overrides/"] {
        if let Some(rest) = normalized.strip_prefix(prefix) {
            return Some(rest.trim_matches('/').to_string());
        }
    }
    None
}

fn write_override_text_file(root: &Path, rel_path: &str, bytes: &[u8]) -> Result<()> {
    let output_path = root.join(rel_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    fs::write(&output_path, bytes)
        .with_context(|| format!("Failed to write {}", output_path.display()))
}

fn is_override_mod_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with("mods/") && normalized.ends_with(".jar")
}

fn resolve_override_mod_entry(
    rel_path: &str,
    loader: &str,
    minecraft_version: &str,
) -> Result<Option<ModEntry>> {
    let query = override_query_for_path(rel_path);
    if query.is_empty() {
        return Ok(None);
    }

    if let Some(entry) =
        resolve_override_with_provider(Provider::Modrinth, &query, loader, minecraft_version)?
    {
        return Ok(Some(entry));
    }

    if let Some(entry) =
        resolve_override_with_provider(Provider::CurseForge, &query, loader, minecraft_version)?
    {
        return Ok(Some(entry));
    }

    Ok(None)
}

fn resolve_override_with_provider(
    provider: Provider,
    query: &str,
    loader: &str,
    minecraft_version: &str,
) -> Result<Option<ModEntry>> {
    let candidates = match mod_resolver::search_blocking(
        provider,
        query,
        loader,
        minecraft_version,
        "mod",
        0,
        10,
    ) {
        Ok(candidates) => candidates,
        Err(error) => {
            eprintln!(
                "{} search failed for '{}': {}",
                super::provider_label(provider),
                query,
                error
            );
            return Ok(None);
        }
    };

    let Some(candidate) = pick_best_override_candidate(&candidates, query) else {
        return Ok(None);
    };

    match mod_resolver::resolve_by_project_id_blocking(
        provider,
        &candidate.project_id,
        loader,
        minecraft_version,
        None,
        "mod",
    ) {
        Ok(resolved) => Ok(Some(resolved.entry)),
        Err(error) => {
            eprintln!(
                "{} resolve failed for '{}': {}",
                super::provider_label(provider),
                candidate.project_id,
                error
            );
            Ok(None)
        }
    }
}

fn pick_best_override_candidate<'a>(
    candidates: &'a [SearchCandidate],
    query: &str,
) -> Option<&'a SearchCandidate> {
    if candidates.is_empty() {
        return None;
    }
    let query_slug = slugify_mod_name(query);
    if let Some(exact) = candidates
        .iter()
        .find(|candidate| slugify_mod_name(&candidate.slug) == query_slug)
    {
        return Some(exact);
    }
    candidates.first()
}

fn slugify_mod_name(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for ch in value.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            slug.push(normalized);
            last_dash = false;
            continue;
        }
        if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn override_query_for_path(path: &str) -> String {
    let file_stem = Path::new(path)
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut query = String::new();
    for token in file_stem
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
    {
        let normalized = token.to_ascii_lowercase();
        if normalized == "mc" {
            continue;
        }
        if let Some(version_token) = normalized.strip_prefix("mc") {
            if !version_token.is_empty() && version_token.chars().all(|ch| ch.is_ascii_digit()) {
                continue;
            }
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        if token
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .count()
            .saturating_mul(2)
            >= token.len()
        {
            continue;
        }
        if !query.is_empty() {
            query.push(' ');
        }
        query.push_str(token);
    }

    if query.trim().is_empty() {
        file_stem
    } else {
        query
    }
}

fn project_url_for_source(source: &str, project_id: &str) -> Option<String> {
    if project_id.trim().is_empty() {
        return None;
    }
    match source {
        "modrinth" => Some(format!("https://modrinth.com/project/{}", project_id)),
        "curseforge" => Some(format!(
            "https://www.curseforge.com/minecraft/mc-mods/{}",
            project_id
        )),
        _ => None,
    }
}

fn parse_mrpack_download_url(url: &str) -> Option<ParsedDownloadUrl> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let segments = parsed
        .path_segments()?
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>();

    if host.contains("modrinth.com") {
        let project_id = path_segment_after(&segments, "data");
        let version = path_segment_after(&segments, "versions")
            .or_else(|| path_segment_after(&segments, "version"));
        return Some(ParsedDownloadUrl {
            source: "modrinth".to_string(),
            project_id,
            version,
            file_id: None,
        });
    }

    if host.contains("curseforge.com") || host.contains("forgecdn.net") {
        let file_id = segments
            .iter()
            .rev()
            .find(|value| value.chars().all(|ch| ch.is_ascii_digit()))
            .cloned();
        return Some(ParsedDownloadUrl {
            source: "curseforge".to_string(),
            project_id: None,
            version: file_id.clone(),
            file_id,
        });
    }

    None
}

fn path_segment_after(segments: &[String], needle: &str) -> Option<String> {
    segments
        .iter()
        .position(|segment| segment == needle)
        .and_then(|index| segments.get(index + 1))
        .cloned()
}

#[derive(Debug, Deserialize)]
struct MrpackIndex {
    #[serde(default)]
    files: Vec<MrpackFile>,
}

#[derive(Debug, Deserialize)]
struct MrpackFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    env: Option<MrpackEnv>,
}

#[derive(Debug, Deserialize)]
struct MrpackEnv {
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    server: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedDownloadUrl {
    source: String,
    project_id: Option<String>,
    version: Option<String>,
    file_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MrpackBuildIndex {
    #[serde(rename = "formatVersion")]
    format_version: u32,
    game: String,
    #[serde(rename = "versionId")]
    version_id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    files: Vec<MrpackBuildFile>,
    dependencies: BTreeMap<String, String>,
    #[serde(rename = "hash-format")]
    hash_format: String,
}

#[derive(Debug, Serialize)]
struct MrpackBuildFile {
    path: String,
    hashes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<MrpackBuildEnv>,
    downloads: Vec<String>,
    #[serde(rename = "fileSize", skip_serializing_if = "Option::is_none")]
    file_size: Option<u64>,
}

#[derive(Debug, Serialize)]
struct MrpackBuildEnv {
    client: String,
    server: String,
}

#[cfg(test)]
mod tests {
    use super::{override_query_for_path, strip_overrides_prefix};

    #[test]
    fn strips_supported_override_prefixes() {
        assert_eq!(
            strip_overrides_prefix("overrides/config/atlas.cfg"),
            Some("config/atlas.cfg".to_string())
        );
        assert_eq!(
            strip_overrides_prefix("client-overrides/kubejs/startup_scripts/a.js"),
            Some("kubejs/startup_scripts/a.js".to_string())
        );
        assert_eq!(
            strip_overrides_prefix("server-overrides/config/common.toml"),
            Some("config/common.toml".to_string())
        );
    }

    #[test]
    fn builds_search_query_from_override_filename() {
        assert_eq!(
            override_query_for_path("mods/create-1.20.1-0.5.1f.jar"),
            "create"
        );
        assert_eq!(
            override_query_for_path("mods/sodium-extra-fabric-0.5.4+mc1.20.1.jar"),
            "sodium extra fabric"
        );
    }
}
