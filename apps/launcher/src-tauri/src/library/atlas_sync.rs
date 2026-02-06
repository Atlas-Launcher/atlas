use crate::launcher::download::download_raw;
use crate::library::error::LibraryError;
use crate::models::{AtlasPackSyncResult, LaunchEvent};
use crate::net::http::shared_client;
use crate::paths::{ensure_dir, normalize_path};
use crate::telemetry;
use protocol::config::mods::{self, ClientOs, ModEntry, ModHashes, ModSide};
use serde::Deserialize;
use sha1::{Digest as Sha1Digest, Sha1};
use sha2::Sha512;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Window};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointerKind {
    Mod,
    Resource,
}

impl PointerKind {
    fn suffix(self) -> &'static str {
        match self {
            Self::Mod => ".mod.toml",
            Self::Resource => ".res.toml",
        }
    }

    fn default_extension(self) -> &'static str {
        match self {
            Self::Mod => ".jar",
            Self::Resource => ".zip",
        }
    }
}

#[derive(Debug, Clone)]
struct PointerFile {
    relative_path: String,
    entry: ModEntry,
    kind: PointerKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactResponse {
    pack_id: String,
    channel: String,
    #[serde(default)]
    build_id: Option<String>,
    #[serde(default)]
    build_version: Option<String>,
    download_url: String,
    #[serde(default)]
    minecraft_version: Option<String>,
    #[serde(default)]
    modloader: Option<String>,
    #[serde(default)]
    modloader_version: Option<String>,
    #[serde(default)]
    force_reinstall: bool,
    #[serde(default)]
    requires_full_reinstall: bool,
}

impl ArtifactResponse {
    fn has_missing_runtime_metadata(&self) -> bool {
        is_blank_option(self.minecraft_version.as_deref())
            || is_blank_option(self.modloader.as_deref())
    }
}

#[derive(Debug, Default)]
struct LastUpdatedState {
    pack_id: String,
    channel: String,
    build_id: Option<String>,
    bundled_files: Option<u64>,
    hydrated_assets: Option<u64>,
    minecraft_version: Option<String>,
    modloader: Option<String>,
    modloader_version: Option<String>,
}

pub async fn sync_atlas_pack(
    window: &Window,
    atlas_hub_url: &str,
    access_token: &str,
    pack_id: &str,
    channel: Option<&str>,
    game_dir: &str,
) -> Result<AtlasPackSyncResult, LibraryError> {
    let requested_channel = normalize_channel(channel);
    let game_dir = normalize_path(game_dir);
    ensure_dir(&game_dir)?;
    let minecraft_dir = game_dir.join(".minecraft");
    ensure_dir(&minecraft_dir)?;
    emit_sync(window, "Checking for pack updates", None, None)?;

    let previous_state = read_last_updated_file(&game_dir);
    let current_build_id = previous_state
        .as_ref()
        .and_then(|state| state.build_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let artifact = fetch_artifact_download(
        atlas_hub_url,
        access_token,
        pack_id,
        requested_channel,
        current_build_id,
    )
    .await?;
    telemetry::info(format!(
        "sync start pack_id={} channel={} build_id={}",
        artifact.pack_id,
        artifact.channel,
        artifact.build_id.as_deref().unwrap_or("-")
    ));
    if let Some(last_updated) = previous_state {
        if sync_is_current(&artifact, &last_updated) {
            let bundled_files = last_updated.bundled_files.unwrap_or(0);
            let hydrated_assets = last_updated.hydrated_assets.unwrap_or(0);
            telemetry::info(format!(
                "sync skipped pack_id={} channel={} build_id={} (already up to date)",
                artifact.pack_id,
                artifact.channel,
                artifact.build_id.as_deref().unwrap_or("-")
            ));
            emit_sync(window, "Pack is already up to date", Some(1), Some(1))?;
            window
                .emit(
                    "launch://status",
                    LaunchEvent {
                        phase: "atlas-sync".to_string(),
                        message: "Pack update complete".to_string(),
                        current: Some(1),
                        total: Some(1),
                        percent: Some(100),
                    },
                )
                .map_err(|err| format!("Emit failed: {err}"))?;
            return Ok(AtlasPackSyncResult {
                pack_id: artifact.pack_id,
                channel: artifact.channel,
                build_id: artifact.build_id,
                build_version: artifact.build_version,
                minecraft_version: first_non_blank(
                    artifact.minecraft_version,
                    last_updated.minecraft_version,
                ),
                modloader: first_non_blank(artifact.modloader, last_updated.modloader),
                modloader_version: first_non_blank(
                    artifact.modloader_version,
                    last_updated.modloader_version,
                ),
                force_reinstall: artifact.force_reinstall,
                requires_full_reinstall: false,
                bundled_files,
                hydrated_assets,
            });
        }
    }

    if artifact.requires_full_reinstall {
        emit_sync(
            window,
            "Force reinstall required by this build; preserving saves",
            None,
            None,
        )?;
        let game_dir_text = game_dir.to_string_lossy().to_string();
        crate::library::uninstall_instance_data(&game_dir_text, true)?;
        ensure_dir(&game_dir)?;
        ensure_dir(&minecraft_dir)?;
    }

    emit_sync(window, "Downloading pack data", None, None)?;
    let blob_bytes = download_blob_bytes(&artifact.download_url).await?;
    telemetry::info(format!(
        "downloaded pack blob pack_id={} bytes={}",
        artifact.pack_id,
        blob_bytes.len()
    ));

    emit_sync(window, "Applying bundled files", None, None)?;
    let blob = protocol::decode_blob(&blob_bytes)
        .map_err(|err| format!("Failed to decode pack blob: {err}"))?;
    let blob_minecraft_version = blob.metadata.minecraft_version.clone();
    let blob_modloader = loader_kind_to_modloader(blob.metadata.loader).to_string();
    let mut expected_mod_paths = HashSet::<PathBuf>::new();

    let mut pointer_files = Vec::new();
    let total_files = blob.files.len() as u64;
    let mut processed_files = 0u64;
    for (relative_path, bytes) in blob.files {
        if let Some(pointer) = parse_pointer_file(&relative_path, &bytes)? {
            pointer_files.push(pointer);
            remove_blob_file_if_exists(&minecraft_dir, &relative_path)?;
        } else {
            let safe_relative = sanitize_relative_path(&relative_path)?;
            if is_mod_relative_path(&safe_relative) {
                expected_mod_paths.insert(safe_relative);
            }
            write_blob_file(&minecraft_dir, &relative_path, &bytes)?;
        }
        processed_files += 1;
        if processed_files % 100 == 0 || processed_files == total_files {
            emit_sync(
                window,
                "Applying bundled files",
                Some(processed_files),
                Some(total_files),
            )?;
        }
    }

    let mut jobs = Vec::new();
    for pointer in pointer_files {
        if !applies_to_this_client(&pointer.entry) {
            telemetry::info(format!(
                "skipping pointer for client constraints: {}",
                pointer.relative_path
            ));
            continue;
        }

        let Some(url) = pointer.entry.download.url.clone() else {
            telemetry::warn(format!(
                "pointer missing download url: {}",
                pointer.relative_path
            ));
            continue;
        };
        if url.trim().is_empty() {
            telemetry::warn(format!(
                "pointer has empty download url: {}",
                pointer.relative_path
            ));
            continue;
        }

        let relative_asset_path =
            destination_relative_path(&pointer.relative_path, pointer.kind, Some(&url));
        jobs.push((
            relative_asset_path,
            url,
            pointer.entry.download.hashes.clone(),
            pointer.kind,
        ));
    }

    let total_assets = jobs.len() as u64;
    if total_assets > 0 {
        emit_sync(
            window,
            "Downloading pack assets",
            Some(0),
            Some(total_assets),
        )?;
    }
    let client = shared_client().clone();
    let mut hydrated_assets = 0u64;
    for (relative_asset_path, url, hashes, kind) in jobs {
        let safe_relative = sanitize_relative_path(&relative_asset_path)?;
        if kind == PointerKind::Mod && is_mod_relative_path(&safe_relative) {
            expected_mod_paths.insert(safe_relative.clone());
        }
        let asset_path = minecraft_dir.join(&safe_relative);
        if asset_path.exists() {
            let can_reuse = match hashes.as_ref() {
                Some(_) => match verify_hashes(&asset_path, hashes.as_ref()) {
                    Ok(()) => true,
                    Err(err) => {
                        telemetry::warn(format!(
                            "asset changed or corrupt; redownloading {}: {}",
                            asset_path.display(),
                            err
                        ));
                        false
                    }
                },
                None => true,
            };
            if can_reuse {
                hydrated_assets += 1;
                emit_sync(
                    window,
                    "Downloading pack assets",
                    Some(hydrated_assets),
                    Some(total_assets),
                )?;
                continue;
            }
        }
        download_raw(&client, &url, &asset_path, None, true).await?;
        verify_hashes(&asset_path, hashes.as_ref())?;
        hydrated_assets += 1;
        emit_sync(
            window,
            "Downloading pack assets",
            Some(hydrated_assets),
            Some(total_assets),
        )?;
    }
    emit_sync(window, "Reconciling mods", None, None)?;
    prune_stale_mods(&minecraft_dir, &expected_mod_paths)?;

    window
        .emit(
            "launch://status",
            LaunchEvent {
                phase: "atlas-sync".to_string(),
                message: "Pack update complete".to_string(),
                current: Some(total_assets),
                total: Some(total_assets),
                percent: Some(100),
            },
        )
        .map_err(|err| format!("Emit failed: {err}"))?;
    telemetry::info(format!(
        "sync complete pack_id={} files={} assets={}",
        artifact.pack_id, processed_files, hydrated_assets
    ));
    if let Err(err) = write_last_updated_file(
        &game_dir,
        &artifact.pack_id,
        &artifact.channel,
        artifact.build_id.as_deref(),
        artifact.build_version.as_deref(),
        artifact.minecraft_version.as_deref(),
        artifact.modloader.as_deref(),
        artifact.modloader_version.as_deref(),
        processed_files,
        hydrated_assets,
    ) {
        telemetry::warn(format!(
            "failed to write last_updated.toml pack_id={} channel={}: {}",
            artifact.pack_id, artifact.channel, err
        ));
    }

    Ok(AtlasPackSyncResult {
        pack_id: artifact.pack_id,
        channel: artifact.channel,
        build_id: artifact.build_id,
        build_version: artifact.build_version,
        minecraft_version: first_non_blank(
            artifact.minecraft_version,
            Some(blob_minecraft_version),
        ),
        modloader: first_non_blank(artifact.modloader, Some(blob_modloader)),
        modloader_version: artifact.modloader_version,
        force_reinstall: artifact.force_reinstall,
        requires_full_reinstall: artifact.requires_full_reinstall,
        bundled_files: processed_files,
        hydrated_assets,
    })
}

async fn fetch_artifact_download(
    atlas_hub_url: &str,
    access_token: &str,
    pack_id: &str,
    requested_channel: &str,
    current_build_id: Option<&str>,
) -> Result<ArtifactResponse, LibraryError> {
    let mut artifact = request_artifact_download(
        atlas_hub_url,
        access_token,
        pack_id,
        requested_channel,
        current_build_id,
    )
    .await?;
    if requested_channel == "production" && artifact.channel.trim().to_lowercase() != "production" {
        return Err(
            "No active Production build is available for this pack. Promote a build to Production before installing or launching."
                .to_string()
                .into(),
        );
    }
    if artifact.has_missing_runtime_metadata() {
        telemetry::warn(format!(
            "artifact metadata missing runtime fields; retrying pack_id={} channel={}",
            pack_id, requested_channel
        ));
        let retry = request_artifact_download(
            atlas_hub_url,
            access_token,
            pack_id,
            requested_channel,
            current_build_id,
        )
        .await?;
        artifact.minecraft_version =
            first_non_blank(artifact.minecraft_version, retry.minecraft_version);
        artifact.modloader = first_non_blank(artifact.modloader, retry.modloader);
        artifact.modloader_version =
            first_non_blank(artifact.modloader_version, retry.modloader_version);
        artifact.force_reinstall = artifact.force_reinstall || retry.force_reinstall;
        artifact.requires_full_reinstall =
            artifact.requires_full_reinstall || retry.requires_full_reinstall;
    }
    if requested_channel == "production" && artifact.channel.trim().to_lowercase() != "production" {
        return Err(
            "No active Production build is available for this pack. Promote a build to Production before installing or launching."
                .to_string()
                .into(),
        );
    }
    Ok(artifact)
}

async fn request_artifact_download(
    atlas_hub_url: &str,
    access_token: &str,
    pack_id: &str,
    channel: &str,
    current_build_id: Option<&str>,
) -> Result<ArtifactResponse, LibraryError> {
    let mut endpoint = Url::parse(&format!(
        "{}/api/launcher/packs/{}/artifact",
        atlas_hub_url.trim_end_matches('/'),
        pack_id
    ))
    .map_err(|err| format!("Invalid artifact endpoint URL: {err}"))?;
    endpoint.query_pairs_mut().append_pair("channel", channel);
    if let Some(value) = current_build_id {
        endpoint
            .query_pairs_mut()
            .append_pair("currentBuildId", value);
    }

    let response = shared_client()
        .get(endpoint.as_str())
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|err| format!("Failed to request artifact metadata: {err}"))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| format!("Failed to read artifact metadata response: {err}"))?;
    if !status.is_success() {
        let text = String::from_utf8_lossy(&body);
        return Err(format!("Artifact metadata request failed ({status}): {text}").into());
    }

    serde_json::from_slice::<ArtifactResponse>(&body).map_err(|err| {
        let body_text = String::from_utf8_lossy(&body);
        format!("Failed to parse artifact metadata JSON: {err}. Body: {body_text}").into()
    })
}

fn normalize_channel(channel: Option<&str>) -> &'static str {
    match channel
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "dev" => "dev",
        "beta" => "beta",
        "production" => "production",
        _ => "production",
    }
}

async fn download_blob_bytes(download_url: &str) -> Result<Vec<u8>, LibraryError> {
    let response = shared_client()
        .get(download_url)
        .send()
        .await
        .map_err(|err| format!("Failed to download pack blob: {err}"))?;

    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| format!("Failed to read pack blob response: {err}"))?;
    if !status.is_success() {
        let text = String::from_utf8_lossy(&body);
        return Err(format!("Pack blob download failed ({status}): {text}").into());
    }

    Ok(body.to_vec())
}

fn write_blob_file(game_dir: &Path, relative_path: &str, bytes: &[u8]) -> Result<(), LibraryError> {
    let safe_relative = sanitize_relative_path(relative_path)?;
    let target_path = game_dir.join(safe_relative);
    if let Some(parent) = target_path.parent() {
        ensure_dir(parent)?;
    }
    if target_path.exists() {
        match fs::read(&target_path) {
            Ok(existing) if existing == bytes => return Ok(()),
            Ok(_) => {}
            Err(err) => {
                telemetry::warn(format!(
                    "failed to read existing bundled file {}; rewriting: {}",
                    target_path.display(),
                    err
                ));
            }
        }
    }
    fs::write(&target_path, bytes).map_err(|err| {
        format!(
            "Failed to write bundled file {}: {err}",
            target_path.display()
        )
    })?;
    Ok(())
}

fn remove_blob_file_if_exists(game_dir: &Path, relative_path: &str) -> Result<(), LibraryError> {
    let safe_relative = sanitize_relative_path(relative_path)?;
    let target_path = game_dir.join(safe_relative);
    if !target_path.exists() {
        return Ok(());
    }

    fs::remove_file(&target_path).map_err(|err| {
        format!(
            "Failed to remove pointer file {}: {err}",
            target_path.display()
        )
    })?;
    Ok(())
}

fn parse_pointer_file(
    relative_path: &str,
    bytes: &[u8],
) -> Result<Option<PointerFile>, LibraryError> {
    if !relative_path.ends_with(".mod.toml") && !relative_path.ends_with(".res.toml") {
        return Ok(None);
    }

    let contents = std::str::from_utf8(bytes)
        .map_err(|err| format!("Pointer file is not valid UTF-8 ({}): {err}", relative_path))?;
    let kind = if relative_path.ends_with(".mod.toml") {
        PointerKind::Mod
    } else {
        PointerKind::Resource
    };

    let entry = match kind {
        PointerKind::Mod => mods::parse_mod_toml(contents)
            .map_err(|err| format!("Invalid mod pointer file {}: {err}", relative_path))?,
        PointerKind::Resource => protocol::config::resources::parse_resource_toml(contents)
            .map_err(|err| format!("Invalid resource pointer file {}: {err}", relative_path))?,
    };

    Ok(Some(PointerFile {
        relative_path: relative_path.to_string(),
        entry,
        kind,
    }))
}

fn sanitize_relative_path(value: &str) -> Result<PathBuf, LibraryError> {
    let normalized = value.replace('\\', "/");
    if normalized.trim().is_empty() {
        return Err("Invalid empty path in pack blob".to_string().into());
    }

    let mut out = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => out.push(part),
            _ => {
                return Err(format!("Invalid relative path in pack blob: {}", normalized).into());
            }
        }
    }

    if out.as_os_str().is_empty() {
        return Err(format!("Invalid relative path in pack blob: {}", normalized).into());
    }

    Ok(out)
}

fn destination_relative_path(pointer_path: &str, kind: PointerKind, url: Option<&str>) -> String {
    let base = pointer_path
        .strip_suffix(kind.suffix())
        .unwrap_or(pointer_path)
        .to_string();

    if base.trim().is_empty() {
        return format!(
            "{}{}",
            match kind {
                PointerKind::Mod => "mods/asset",
                PointerKind::Resource => "resources/asset",
            },
            kind.default_extension()
        );
    }

    if Path::new(&base).extension().is_some() {
        return base;
    }

    let extension = extension_from_url(url).unwrap_or_else(|| kind.default_extension().to_string());
    format!("{}{}", base, extension)
}

fn extension_from_url(url: Option<&str>) -> Option<String> {
    let value = url?;
    let parsed = Url::parse(value).ok()?;
    let last = parsed
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or_default();
    if last.is_empty() {
        return None;
    }

    let ext = Path::new(last).extension()?.to_str()?.to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 10 || !ext.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    Some(format!(".{}", ext))
}

fn is_mod_relative_path(relative_path: &Path) -> bool {
    matches!(
        relative_path.components().next(),
        Some(Component::Normal(segment)) if segment.to_string_lossy().eq_ignore_ascii_case("mods")
    )
}

fn prune_stale_mods(
    minecraft_dir: &Path,
    expected_mod_paths: &HashSet<PathBuf>,
) -> Result<(), LibraryError> {
    let mods_dir = minecraft_dir.join("mods");
    if !mods_dir.exists() {
        return Ok(());
    }

    let mut stack = vec![mods_dir.clone()];
    let mut removed_files = 0u64;
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("Failed to read mods directory {}: {err}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("Failed to read mods directory entry: {err}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path.strip_prefix(minecraft_dir).map_err(|err| {
                format!(
                    "Failed to resolve mod path {} relative to {}: {err}",
                    path.display(),
                    minecraft_dir.display()
                )
            })?;
            let normalized = sanitize_relative_path(&relative.to_string_lossy())?;
            if expected_mod_paths.contains(&normalized) {
                continue;
            }
            fs::remove_file(&path)
                .map_err(|err| format!("Failed to remove stale mod {}: {err}", path.display()))?;
            removed_files += 1;
            telemetry::info(format!(
                "removed stale mod file not present in latest blob: {}",
                path.display()
            ));
        }
    }

    prune_empty_mod_subdirs(&mods_dir)?;
    telemetry::info(format!(
        "mod reconciliation complete root={} expected={} removed={}",
        mods_dir.display(),
        expected_mod_paths.len(),
        removed_files
    ));
    Ok(())
}

fn prune_empty_mod_subdirs(mods_dir: &Path) -> Result<(), LibraryError> {
    if !mods_dir.exists() {
        return Ok(());
    }

    let mut dirs = Vec::<PathBuf>::new();
    let mut stack = vec![mods_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        dirs.push(dir.clone());
        let entries = fs::read_dir(&dir)
            .map_err(|err| format!("Failed to read mods directory {}: {err}", dir.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("Failed to read mods directory entry: {err}"))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            }
        }
    }

    dirs.sort_by_key(|dir| std::cmp::Reverse(dir.components().count()));
    for dir in dirs {
        if dir == mods_dir {
            continue;
        }
        let mut entries = fs::read_dir(&dir)
            .map_err(|err| format!("Failed to read mods directory {}: {err}", dir.display()))?;
        if entries.next().is_none() {
            fs::remove_dir(&dir).map_err(|err| {
                format!("Failed to remove empty directory {}: {err}", dir.display())
            })?;
        }
    }
    Ok(())
}

fn applies_to_this_client(entry: &ModEntry) -> bool {
    match entry.metadata.side {
        ModSide::Server => return false,
        ModSide::Client | ModSide::Both => {}
    }

    match current_client_os() {
        Some(os) => !entry.metadata.disabled_client_oses.contains(&os),
        None => true,
    }
}

fn current_client_os() -> Option<ClientOs> {
    match std::env::consts::OS {
        "macos" => Some(ClientOs::Macos),
        "windows" => Some(ClientOs::Windows),
        "linux" => Some(ClientOs::Linux),
        _ => None,
    }
}

fn verify_hashes(path: &Path, hashes: Option<&ModHashes>) -> Result<(), LibraryError> {
    let Some(hashes) = hashes else {
        return Ok(());
    };

    if let Some(expected_sha1) = hashes
        .sha1
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let actual = sha1_file(path)?;
        if !actual.eq_ignore_ascii_case(expected_sha1) {
            return Err(format!(
                "SHA-1 mismatch for {} (expected {}, got {})",
                path.display(),
                expected_sha1,
                actual
            )
            .into());
        }
    }

    if let Some(expected_sha512) = hashes
        .sha512
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        let actual = sha512_file(path)?;
        if !actual.eq_ignore_ascii_case(expected_sha512) {
            return Err(format!(
                "SHA-512 mismatch for {} (expected {}, got {})",
                path.display(),
                expected_sha512,
                actual
            )
            .into());
        }
    }

    Ok(())
}

fn sha1_file(path: &Path) -> Result<String, LibraryError> {
    let bytes = fs::read(path)
        .map_err(|err| format!("Failed to read {} for SHA-1: {err}", path.display()))?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn sha512_file(path: &Path) -> Result<String, LibraryError> {
    let bytes = fs::read(path)
        .map_err(|err| format!("Failed to read {} for SHA-512: {err}", path.display()))?;
    let mut hasher = Sha512::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn emit_sync(
    window: &Window,
    message: impl Into<String>,
    current: Option<u64>,
    total: Option<u64>,
) -> Result<(), LibraryError> {
    window
        .emit(
            "launch://status",
            LaunchEvent {
                phase: "atlas-sync".to_string(),
                message: message.into(),
                current,
                total,
                percent: None,
            },
        )
        .map_err(|err| format!("Emit failed: {err}").into())
}

fn sync_is_current(artifact: &ArtifactResponse, state: &LastUpdatedState) -> bool {
    let Some(artifact_build_id) = artifact
        .build_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };

    let local_build_id = state
        .build_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if local_build_id != Some(artifact_build_id) {
        return false;
    }

    if state.pack_id.trim() != artifact.pack_id.trim() {
        return false;
    }
    state
        .channel
        .trim()
        .eq_ignore_ascii_case(artifact.channel.trim())
}

fn read_last_updated_file(game_dir: &Path) -> Option<LastUpdatedState> {
    let metadata_path = game_dir.join("last_updated.toml");
    let contents = fs::read_to_string(&metadata_path).ok()?;
    let mut state = LastUpdatedState::default();
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "pack_id" => state.pack_id = parse_toml_string(value).unwrap_or_default(),
            "channel" => state.channel = parse_toml_string(value).unwrap_or_default(),
            "build_id" => state.build_id = parse_toml_string(value),
            "bundled_files" => state.bundled_files = value.parse::<u64>().ok(),
            "hydrated_assets" => state.hydrated_assets = value.parse::<u64>().ok(),
            "minecraft_version" => state.minecraft_version = parse_toml_string(value),
            "modloader" => state.modloader = parse_toml_string(value),
            "modloader_version" => state.modloader_version = parse_toml_string(value),
            _ => {}
        }
    }
    if state.pack_id.trim().is_empty() || state.channel.trim().is_empty() {
        return None;
    }
    Some(state)
}

fn parse_toml_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("null") {
        return None;
    }
    let inner = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    Some(toml_unescape_string(inner))
}

fn toml_unescape_string(value: &str) -> String {
    let mut chars = value.chars().peekable();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn write_last_updated_file(
    game_dir: &Path,
    pack_id: &str,
    channel: &str,
    build_id: Option<&str>,
    build_version: Option<&str>,
    minecraft_version: Option<&str>,
    modloader: Option<&str>,
    modloader_version: Option<&str>,
    bundled_files: u64,
    hydrated_assets: u64,
) -> Result<(), LibraryError> {
    ensure_dir(game_dir)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("Failed to read system clock: {err}"))?
        .as_secs();

    let payload = format!(
        concat!(
            "updated_at_unix = {updated_at}\n",
            "pack_id = \"{pack_id}\"\n",
            "channel = \"{channel}\"\n",
            "build_id = {build_id}\n",
            "build_version = {build_version}\n",
            "minecraft_version = {minecraft_version}\n",
            "modloader = {modloader}\n",
            "modloader_version = {modloader_version}\n",
            "bundled_files = {bundled_files}\n",
            "hydrated_assets = {hydrated_assets}\n"
        ),
        updated_at = timestamp,
        pack_id = toml_escape_string(pack_id),
        channel = toml_escape_string(channel),
        build_id = toml_optional_string(build_id),
        build_version = toml_optional_string(build_version),
        minecraft_version = toml_optional_string(minecraft_version),
        modloader = toml_optional_string(modloader),
        modloader_version = toml_optional_string(modloader_version),
        bundled_files = bundled_files,
        hydrated_assets = hydrated_assets
    );

    let metadata_path = game_dir.join("last_updated.toml");
    fs::write(&metadata_path, payload)
        .map_err(|err| format!("Failed to write {}: {err}", metadata_path.display()).into())
}

fn toml_optional_string(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => format!("\"{}\"", toml_escape_string(v)),
        None => "null".to_string(),
    }
}

fn toml_escape_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\\' => ['\\', '\\'].into_iter().collect::<Vec<char>>(),
            '"' => ['\\', '"'].into_iter().collect::<Vec<char>>(),
            '\n' => ['\\', 'n'].into_iter().collect::<Vec<char>>(),
            '\r' => ['\\', 'r'].into_iter().collect::<Vec<char>>(),
            '\t' => ['\\', 't'].into_iter().collect::<Vec<char>>(),
            _ => [ch].into_iter().collect::<Vec<char>>(),
        })
        .collect()
}

fn is_blank_option(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}

fn first_non_blank(first: Option<String>, second: Option<String>) -> Option<String> {
    match first {
        Some(value) if !value.trim().is_empty() => Some(value),
        _ => second.filter(|value| !value.trim().is_empty()),
    }
}

fn loader_kind_to_modloader(loader: protocol::Loader) -> &'static str {
    match loader {
        protocol::Loader::Fabric => "fabric",
        protocol::Loader::Forge => "forge",
        protocol::Loader::Neo => "neoforge",
    }
}
