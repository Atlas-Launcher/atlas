use crate::models::{AuthSession, LaunchEvent, LaunchOptions};
use crate::paths::{ensure_dir, file_exists, normalize_path};
use futures::stream::{self, StreamExt};
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Window;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use zip::ZipArchive;

const VERSION_MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest.json";
const JAVA_RUNTIME_MANIFEST_URL: &str =
  "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";
const DOWNLOAD_CONCURRENCY: usize = 12;

#[derive(Debug, Deserialize, Serialize)]
struct VersionManifest {
  latest: LatestVersion,
  versions: Vec<VersionRef>
}

#[derive(Debug, Deserialize, Serialize)]
struct LatestVersion {
  release: String,
  #[allow(dead_code)]
  snapshot: String
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct VersionRef {
  id: String,
  #[serde(rename = "type")]
  kind: String,
  url: String
}

#[derive(Debug, Deserialize, Serialize)]
struct VersionData {
  id: String,
  #[serde(rename = "type")]
  kind: String,
  #[serde(rename = "mainClass")]
  main_class: String,
  #[serde(default)]
  arguments: Option<Arguments>,
  #[serde(default, rename = "minecraftArguments")]
  minecraft_arguments: Option<String>,
  #[serde(rename = "assetIndex")]
  asset_index: AssetIndex,
  downloads: VersionDownloads,
  libraries: Vec<Library>,
  #[serde(default, rename = "javaVersion")]
  java_version: Option<JavaVersion>
}

#[derive(Debug, Deserialize, Serialize)]
struct VersionDownloads {
  client: Download
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Download {
  #[serde(default)]
  path: Option<String>,
  url: String,
  #[serde(default)]
  sha1: Option<String>,
  #[serde(default)]
  size: Option<u64>
}

#[derive(Debug, Deserialize, Serialize)]
struct AssetIndex {
  id: String,
  url: String,
  #[serde(default)]
  sha1: Option<String>,
  #[serde(default)]
  size: Option<u64>
}

#[derive(Debug, Deserialize, Serialize)]
struct JavaVersion {
  component: String,
  #[serde(rename = "majorVersion")]
  major_version: u32
}

#[derive(Debug, Deserialize, Serialize)]
struct AssetIndexData {
  objects: HashMap<String, AssetObject>
}

#[derive(Debug, Deserialize, Serialize)]
struct AssetObject {
  hash: String,
  #[allow(dead_code)]
  size: u64
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Library {
  name: String,
  #[serde(default)]
  downloads: Option<LibraryDownloads>,
  #[serde(default)]
  natives: Option<HashMap<String, String>>,
  #[serde(default)]
  rules: Option<Vec<Rule>>,
  #[serde(default)]
  extract: Option<Extract>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LibraryDownloads {
  #[serde(default)]
  artifact: Option<Download>,
  #[serde(default)]
  classifiers: Option<HashMap<String, Download>>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Extract {
  #[serde(default)]
  exclude: Vec<String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Arguments {
  #[serde(default)]
  game: Vec<Argument>,
  #[serde(default)]
  jvm: Vec<Argument>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum Argument {
  String(String),
  Rule { rules: Vec<Rule>, value: ArgValue }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum ArgValue {
  String(String),
  List(Vec<String>)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Rule {
  action: String,
  #[serde(default)]
  os: Option<RuleOs>,
  #[serde(default)]
  features: Option<HashMap<String, bool>>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RuleOs {
  #[serde(default)]
  name: Option<String>
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeFiles {
  files: HashMap<String, JavaRuntimeFile>
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeFile {
  #[serde(rename = "type")]
  kind: String,
  #[serde(default)]
  executable: bool,
  #[serde(default)]
  downloads: Option<JavaRuntimeDownloads>,
  #[serde(default)]
  target: Option<String>
}

#[derive(Debug, Deserialize)]
struct JavaRuntimeDownloads {
  #[serde(default)]
  raw: Option<Download>
}

struct PreparedMinecraft {
  game_dir: PathBuf,
  assets_dir: PathBuf,
  version_data: VersionData,
  client_jar_path: PathBuf,
  library_paths: Vec<PathBuf>,
  natives_dir: PathBuf,
  java_path: String
}

async fn prepare_minecraft(
  window: &Window,
  options: &LaunchOptions
) -> Result<PreparedMinecraft, String> {
  let client = Client::new();
  let game_dir = normalize_path(&options.game_dir);
  let versions_dir = game_dir.join("versions");
  let libraries_dir = game_dir.join("libraries");
  let assets_dir = game_dir.join("assets");
  ensure_dir(&versions_dir)?;
  ensure_dir(&libraries_dir)?;
  ensure_dir(&assets_dir.join("indexes"))?;
  ensure_dir(&assets_dir.join("objects"))?;

  emit(window, "setup", "Fetching version manifest", None, None)?;
  let manifest: VersionManifest = fetch_json(&client, VERSION_MANIFEST_URL).await?;

  let version_id = options
    .version
    .clone()
    .unwrap_or_else(|| manifest.latest.release.clone());

  let version_ref = manifest
    .versions
    .into_iter()
    .find(|version| version.id == version_id)
    .ok_or_else(|| format!("Version {version_id} not found in manifest"))?;

  emit(window, "setup", format!("Downloading version metadata ({})", version_ref.id), None, None)?;
  let version_data: VersionData = fetch_json(&client, &version_ref.url).await?;
  let version_folder = versions_dir.join(&version_data.id);
  ensure_dir(&version_folder)?;

  let version_json_path = version_folder.join(format!("{}.json", version_data.id));
  let version_bytes = serde_json::to_vec_pretty(&version_data)
    .map_err(|err| format!("Failed to serialize version metadata: {err}"))?;
  fs::write(&version_json_path, version_bytes)
    .map_err(|err| format!("Failed to write version metadata: {err}"))?;

  emit(window, "client", "Downloading client jar", None, None)?;
  let client_jar_path = version_folder.join(format!("{}.jar", version_data.id));
  download_if_needed(&client, &version_data.downloads.client, &client_jar_path).await?;

  emit(window, "libraries", "Syncing libraries", None, None)?;
  let (library_paths, native_jars) = sync_libraries(
    &client,
    &libraries_dir,
    &version_data.libraries,
    window
  )
  .await?;

  emit(window, "natives", "Extracting natives", None, None)?;
  let natives_dir = version_folder.join("natives");
  if natives_dir.exists() {
    fs::remove_dir_all(&natives_dir).map_err(|err| format!("Failed to clear natives: {err}"))?;
  }
  ensure_dir(&natives_dir)?;
  for native in native_jars {
    extract_natives(&native, &natives_dir, &version_data.libraries)?;
  }

  emit(window, "assets", "Syncing assets", None, None)?;
  let assets_index_path = assets_dir
    .join("indexes")
    .join(format!("{}.json", version_data.asset_index.id));
  download_if_needed(&client, &Download {
    path: None,
    url: version_data.asset_index.url.clone(),
    sha1: version_data.asset_index.sha1.clone(),
    size: version_data.asset_index.size
  }, &assets_index_path).await?;

  let assets_index_data: AssetIndexData = serde_json::from_slice(
    &fs::read(&assets_index_path).map_err(|err| format!("Failed to read asset index: {err}"))?
  )
  .map_err(|err| format!("Failed to parse asset index: {err}"))?;

  let total_assets = assets_index_data.objects.len() as u64;
  let mut processed_assets = 0u64;
  let mut asset_jobs: Vec<(String, PathBuf, u64)> = Vec::new();
  for (_name, asset) in assets_index_data.objects.iter() {
    let hash = &asset.hash;
    let sub = &hash[0..2];
    let object_path = assets_dir.join("objects").join(sub).join(hash);
    if file_exists(&object_path) {
      processed_assets += 1;
      if processed_assets % 250 == 0 || processed_assets == total_assets {
        emit(
          window,
          "assets",
          format!("Assets {processed_assets}/{total_assets}"),
          Some(processed_assets),
          Some(total_assets)
        )?;
      }
      continue;
    }
    let url = format!("https://resources.download.minecraft.net/{}/{}", sub, hash);
    asset_jobs.push((url, object_path, asset.size));
  }

  if !asset_jobs.is_empty() {
    let mut stream = stream::iter(asset_jobs.into_iter().map(|(url, path, size)| {
      let client = client.clone();
      async move {
        if let Some(parent) = path.parent() {
          ensure_dir(parent)?;
        }
        download_raw(&client, &url, &path, Some(size), true).await
      }
    }))
    .buffer_unordered(DOWNLOAD_CONCURRENCY);

    while let Some(result) = stream.next().await {
      result?;
      processed_assets += 1;
      if processed_assets % 250 == 0 || processed_assets == total_assets {
        emit(
          window,
          "assets",
          format!("Assets {processed_assets}/{total_assets}"),
          Some(processed_assets),
          Some(total_assets)
        )?;
      }
    }
  }

  let java_path =
    resolve_java_path(window, &game_dir, &version_data, &options.java_path).await?;

  Ok(PreparedMinecraft {
    game_dir,
    assets_dir,
    version_data,
    client_jar_path,
    library_paths,
    natives_dir,
    java_path
  })
}

pub async fn launch_minecraft(
  window: &Window,
  options: &LaunchOptions,
  session: &AuthSession
) -> Result<(), String> {
  let prepared = prepare_minecraft(window, options).await?;
  let game_dir = prepared.game_dir;
  let assets_dir = prepared.assets_dir;
  let version_data = prepared.version_data;
  let client_jar_path = prepared.client_jar_path;
  let library_paths = prepared.library_paths;
  let natives_dir = prepared.natives_dir;
  let java_path = prepared.java_path;

  emit(window, "launch", "Preparing JVM arguments", None, None)?;
  let classpath = build_classpath(&library_paths, &client_jar_path);

  let mut replace_map = HashMap::new();
  replace_map.insert("auth_player_name", session.profile.name.clone());
  replace_map.insert("version_name", version_data.id.clone());
  replace_map.insert("game_directory", game_dir.to_string_lossy().to_string());
  replace_map.insert("assets_root", assets_dir.to_string_lossy().to_string());
  replace_map.insert("assets_index_name", version_data.asset_index.id.clone());
  replace_map.insert("auth_uuid", session.profile.id.clone());
  replace_map.insert("auth_access_token", session.access_token.clone());
  replace_map.insert("user_type", "msa".to_string());
  replace_map.insert("version_type", version_data.kind.clone());
  replace_map.insert("classpath", classpath.clone());
  replace_map.insert("natives_directory", natives_dir.to_string_lossy().to_string());
  replace_map.insert("launcher_name", "mc-launcher".to_string());
  replace_map.insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());

  let (mut jvm_args, game_args) = build_arguments(&version_data, &replace_map)?;

  let memory = options.memory_mb.max(1024);
  let mem_arg = format!("-Xmx{}M", memory);
  jvm_args.insert(0, mem_arg);
  jvm_args.insert(1, "-Xms512M".into());

  if !jvm_args.iter().any(|arg| arg.contains("-Djava.library.path")) {
    jvm_args.push(format!("-Djava.library.path={}", natives_dir.to_string_lossy()));
  }

  emit(window, "launch", "Spawning Minecraft", None, None)?;
  let mut command = Command::new(java_path);
  command
    .current_dir(&game_dir)
    .args(&jvm_args)
    .arg(&version_data.main_class)
    .args(&game_args);

  command
    .spawn()
    .map_err(|err| format!("Failed to launch Minecraft: {err}"))?;

  emit(window, "launch", "Minecraft process started", None, None)?;
  Ok(())
}

pub async fn download_minecraft_files(
  window: &Window,
  options: &LaunchOptions
) -> Result<(), String> {
  prepare_minecraft(window, options).await?;
  emit(window, "download", "Minecraft files are ready", None, None)?;
  Ok(())
}

fn emit(
  window: &Window,
  phase: &str,
  message: impl Into<String>,
  current: Option<u64>,
  total: Option<u64>
) -> Result<(), String> {
  window
    .emit(
      "launch://status",
      LaunchEvent {
        phase: phase.into(),
        message: message.into(),
        current,
        total,
        percent: None
      }
    )
    .map_err(|err| format!("Emit failed: {err}"))
}

async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, String> {
  let response = client
    .get(url)
    .send()
    .await
    .map_err(|err| format!("Request failed: {err}"))?;

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Request failed ({status}): {text}"));
  }

  response
    .json::<T>()
    .await
    .map_err(|err| format!("Failed to parse JSON: {err}"))
}

async fn download_if_needed(client: &Client, download: &Download, path: &Path) -> Result<(), String> {
  let mut allow_resume = true;
  if file_exists(path) {
    if let Some(expected) = &download.sha1 {
      if let Ok(actual) = sha1_file(path) {
        if &actual == expected {
          return Ok(());
        }
      }
      // Corrupt or mismatched checksum: force full download.
      allow_resume = false;
    }

    if allow_resume {
      if let Some(expected_size) = download.size {
        if let Ok(actual_size) = std::fs::metadata(path).map(|m| m.len()) {
          if actual_size == expected_size {
            return Ok(());
          }
        }
      }
    }
  }

  download_raw(client, &download.url, path, download.size, allow_resume).await
}

async fn download_raw(
  client: &Client,
  url: &str,
  path: &Path,
  expected_size: Option<u64>,
  allow_resume: bool
) -> Result<(), String> {
  let mut existing = if allow_resume && file_exists(path) {
    std::fs::metadata(path)
      .map(|m| m.len())
      .unwrap_or(0)
  } else {
    0
  };

  if let Some(size) = expected_size {
    if existing >= size {
      return Ok(());
    }
  }

  if let Some(parent) = path.parent() {
    ensure_dir(parent)?;
  }

  let mut request = client.get(url);
  if allow_resume && existing > 0 {
    request = request.header(RANGE, format!("bytes={}-", existing));
  }

  let mut response = request
    .send()
    .await
    .map_err(|err| format!("Download failed: {err}"))?;

  if allow_resume && existing > 0 {
    match response.status() {
      StatusCode::PARTIAL_CONTENT => {}
      StatusCode::RANGE_NOT_SATISFIABLE => {
        if let Some(size) = expected_size {
          if existing == size {
            return Ok(());
          }
        }
        existing = 0;
        response = client
          .get(url)
          .send()
          .await
          .map_err(|err| format!("Download failed: {err}"))?;
      }
      status if status.is_success() => {
        existing = 0;
      }
      status => {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("Download failed ({status}): {text}"));
      }
    }
  }

  if !response.status().is_success() {
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    return Err(format!("Download failed ({status}): {text}"));
  }

  let mut file = if allow_resume && existing > 0 && response.status() == StatusCode::PARTIAL_CONTENT {
    async_fs::OpenOptions::new()
      .append(true)
      .open(path)
      .await
      .map_err(|err| format!("Failed to open file for resume: {err}"))?
  } else {
    async_fs::File::create(path)
      .await
      .map_err(|err| format!("Failed to write file: {err}"))?
  };

  let mut stream = response.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let bytes = chunk.map_err(|err| format!("Failed to read download: {err}"))?;
    file
      .write_all(&bytes)
      .await
      .map_err(|err| format!("Failed to write file: {err}"))?;
  }

  file
    .flush()
    .await
    .map_err(|err| format!("Failed to flush download: {err}"))?;

  if let Some(size) = expected_size {
    if let Ok(actual) = std::fs::metadata(path).map(|m| m.len()) {
      if actual != size {
        return Err(format!(
          "Download incomplete: expected {size} bytes, got {actual} bytes"
        ));
      }
    }
  }

  Ok(())
}

fn sha1_file(path: &Path) -> Result<String, String> {
  let mut file = File::open(path).map_err(|err| format!("Failed to open file: {err}"))?;
  let mut hasher = Sha1::new();
  let mut buffer = [0u8; 8192];
  loop {
    let read = file.read(&mut buffer).map_err(|err| format!("Read failed: {err}"))?;
    if read == 0 {
      break;
    }
    hasher.update(&buffer[..read]);
  }
  Ok(hex::encode(hasher.finalize()))
}

async fn sync_libraries(
  client: &Client,
  libraries_dir: &Path,
  libraries: &[Library],
  window: &Window
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
  let mut library_paths = Vec::new();
  let mut native_paths = Vec::new();
  let mut downloads: Vec<(Download, PathBuf)> = Vec::new();

  let included: Vec<Library> = libraries
    .iter()
    .cloned()
    .filter(|lib| rules_allow(&lib.rules))
    .collect();

  let os_key = current_os_key();
  let arch = current_arch();

  for library in included {
    if let Some(downloads_entry) = &library.downloads {
      if let Some(artifact) = &downloads_entry.artifact {
        let path = libraries_dir.join(
          artifact
            .path
            .clone()
            .unwrap_or_else(|| library_path_from_name(&library.name))
        );
        library_paths.push(path);
        downloads.push((artifact.clone(), library_paths.last().unwrap().clone()));
      }

      if let Some(natives) = &library.natives {
        if let Some(classifier) = natives.get(os_key) {
          let classifier = classifier.replace("${arch}", arch);
          if let Some(classifiers) = &downloads_entry.classifiers {
            if let Some(native) = classifiers.get(&classifier) {
              let path = libraries_dir.join(
                native
                  .path
                  .clone()
                  .unwrap_or_else(|| library_path_from_name(&library.name))
              );
              native_paths.push(path);
              downloads.push((native.clone(), native_paths.last().unwrap().clone()));
            }
          }
        }
      }
    }
  }

  let total = downloads.len() as u64;
  let mut index = 0u64;
  if total > 0 {
    let mut stream = stream::iter(downloads.into_iter().map(|(download, path)| {
      let client = client.clone();
      async move { download_if_needed(&client, &download, &path).await }
    }))
    .buffer_unordered(DOWNLOAD_CONCURRENCY);

    while let Some(result) = stream.next().await {
      result?;
      index += 1;
      if index % 10 == 0 || index == total {
        emit(
          window,
          "libraries",
          format!("Libraries {index}/{total}"),
          Some(index),
          Some(total)
        )?;
      }
    }
  }

  Ok((library_paths, native_paths))
}

fn extract_natives(path: &Path, natives_dir: &Path, libraries: &[Library]) -> Result<(), String> {
  let file = File::open(path).map_err(|err| format!("Failed to open native jar: {err}"))?;
  let mut archive = ZipArchive::new(file).map_err(|err| format!("Failed to read native jar: {err}"))?;

  let mut excluded = Vec::new();
  for lib in libraries {
    if let Some(extract) = &lib.extract {
      excluded.extend(extract.exclude.iter().cloned());
    }
  }

  for i in 0..archive.len() {
    let mut entry = archive.by_index(i).map_err(|err| format!("Zip error: {err}"))?;
    let name = entry.name().to_string();
    if name.starts_with("META-INF/") {
      continue;
    }
    if excluded.iter().any(|pattern| name.starts_with(pattern)) {
      continue;
    }
    if entry.is_dir() {
      continue;
    }

    let out_path = natives_dir.join(name);
    if let Some(parent) = out_path.parent() {
      ensure_dir(parent)?;
    }
    let mut outfile = File::create(&out_path).map_err(|err| format!("Failed to write native: {err}"))?;
    std::io::copy(&mut entry, &mut outfile).map_err(|err| format!("Failed to extract native: {err}"))?;
  }

  Ok(())
}

fn build_classpath(libraries: &[PathBuf], client_jar: &Path) -> String {
  let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
  let mut entries: Vec<String> = libraries.iter().map(|path| path.to_string_lossy().to_string()).collect();
  entries.push(client_jar.to_string_lossy().to_string());
  entries.join(sep)
}

fn build_arguments(
  version: &VersionData,
  replacements: &HashMap<&str, String>
) -> Result<(Vec<String>, Vec<String>), String> {
  if let Some(arguments) = &version.arguments {
    let jvm = expand_args(&arguments.jvm, replacements);
    let game = expand_args(&arguments.game, replacements);
    return Ok((jvm, game));
  }

  let raw = version
    .minecraft_arguments
    .clone()
    .ok_or_else(|| "Missing arguments in version metadata".to_string())?;
  let game = raw
    .split_whitespace()
    .map(|arg| replace_tokens(arg, replacements))
    .collect::<Vec<_>>();

  Ok((Vec::new(), game))
}

fn expand_args(args: &[Argument], replacements: &HashMap<&str, String>) -> Vec<String> {
  let mut expanded = Vec::new();
  for arg in args {
    match arg {
      Argument::String(value) => expanded.push(replace_tokens(value, replacements)),
      Argument::Rule { rules, value } => {
        if rules_allow(&Some(rules.clone())) {
          match value {
            ArgValue::String(value) => expanded.push(replace_tokens(value, replacements)),
            ArgValue::List(list) => {
              for item in list {
                expanded.push(replace_tokens(item, replacements));
              }
            }
          }
        }
      }
    }
  }
  expanded
}

fn replace_tokens(input: &str, replacements: &HashMap<&str, String>) -> String {
  let mut output = input.to_string();
  for (key, value) in replacements {
    output = output.replace(&format!("${{{}}}", key), value);
  }
  output
}

fn rules_allow(rules: &Option<Vec<Rule>>) -> bool {
  let Some(rules) = rules else {
    return true;
  };

  let mut allowed = false;
  for rule in rules {
    let os_applies = rule
      .os
      .as_ref()
      .and_then(|os| os.name.as_ref())
      .map(|name| name == current_os_key())
      .unwrap_or(true);

    let features_applies = features_match(rule.features.as_ref());
    let applies = os_applies && features_applies;

    if applies {
      allowed = rule.action == "allow";
    }
  }
  allowed
}

fn features_match(features: Option<&HashMap<String, bool>>) -> bool {
  let Some(features) = features else {
    return true;
  };

  let supported = current_features();
  for (key, expected) in features {
    let actual = supported.get(key).copied().unwrap_or(false);
    if actual != *expected {
      return false;
    }
  }
  true
}

fn current_features() -> HashMap<String, bool> {
  let mut features = HashMap::new();
  features.insert("is_demo_user".to_string(), false);
  features.insert("has_custom_resolution".to_string(), false);
  features.insert("has_quick_plays_support".to_string(), false);
  features.insert("is_quick_play_singleplayer".to_string(), false);
  features.insert("is_quick_play_multiplayer".to_string(), false);
  features.insert("is_quick_play_realms".to_string(), false);
  features
}

fn library_path_from_name(name: &str) -> String {
  let parts: Vec<&str> = name.split(':').collect();
  if parts.len() < 3 {
    return name.replace(':', "/");
  }
  let group = parts[0].replace('.', "/");
  let artifact = parts[1];
  let version = parts[2];
  let classifier = parts.get(3).copied();

  let filename = if let Some(classifier) = classifier {
    format!("{}-{}-{}.jar", artifact, version, classifier)
  } else {
    format!("{}-{}.jar", artifact, version)
  };

  format!("{}/{}/{}/{}", group, artifact, version, filename)
}

fn current_os_key() -> &'static str {
  if cfg!(target_os = "windows") {
    "windows"
  } else if cfg!(target_os = "macos") {
    "osx"
  } else {
    "linux"
  }
}

fn current_arch() -> &'static str {
  if cfg!(target_arch = "x86_64") {
    "64"
  } else if cfg!(target_arch = "x86") {
    "32"
  } else if cfg!(target_arch = "aarch64") {
    "arm64"
  } else {
    "64"
  }
}

fn runtime_os_key() -> Result<&'static str, String> {
  if cfg!(target_os = "windows") {
    return Ok(match current_arch() {
      "64" => "windows-x64",
      "32" => "windows-x86",
      "arm64" => "windows-arm64",
      _ => "windows-x64"
    });
  }
  if cfg!(target_os = "macos") {
    return Ok(match current_arch() {
      "arm64" => "mac-os-arm64",
      _ => "mac-os"
    });
  }
  if cfg!(target_os = "linux") {
    return Ok(match current_arch() {
      "32" => "linux-i386",
      "arm64" => "linux-arm64",
      _ => "linux"
    });
  }
  Err("Unsupported OS for Java runtime downloads.".to_string())
}

fn select_java_component(
  platform: &serde_json::Map<String, serde_json::Value>,
  desired: &str
) -> String {
  if platform
    .get(desired)
    .and_then(|value| value.as_array())
    .map(|items| !items.is_empty())
    .unwrap_or(false)
  {
    return desired.to_string();
  }

  let mut candidates = vec![
    "java-runtime-delta",
    "java-runtime-gamma",
    "java-runtime-beta",
    "java-runtime-alpha",
    "jre-legacy"
  ];

  if !candidates.iter().any(|item| *item == desired) {
    candidates.insert(0, desired);
  }

  for candidate in candidates {
    if platform
      .get(candidate)
      .and_then(|value| value.as_array())
      .map(|items| !items.is_empty())
      .unwrap_or(false)
    {
      return candidate.to_string();
    }
  }

  platform.keys().next().cloned().unwrap_or_else(|| desired.to_string())
}

async fn resolve_java_path(
  window: &Window,
  game_dir: &Path,
  version_data: &VersionData,
  java_path_override: &str
) -> Result<String, String> {
  if !java_path_override.trim().is_empty() && java_path_override.trim() != "java" {
    return Ok(java_path_override.trim().to_string());
  }
  let component = version_data
    .java_version
    .as_ref()
    .map(|java| java.component.clone())
    .unwrap_or_else(|| "jre-legacy".to_string());

  ensure_java_runtime(window, game_dir, &component).await
}

async fn ensure_java_runtime(
  window: &Window,
  game_dir: &Path,
  component: &str
) -> Result<String, String> {
  let client = Client::new();
  let os_key = runtime_os_key()?;

  emit(
    window,
    "java",
    format!("Checking Java runtime ({component})"),
    None,
    None
  )?;

  let manifest: serde_json::Value = fetch_json(&client, JAVA_RUNTIME_MANIFEST_URL).await?;
  let platform = manifest
    .get(os_key)
    .and_then(|value| value.as_object())
    .ok_or_else(|| format!("Java runtime platform {os_key} not found"))?;

  let chosen_component = select_java_component(platform, component);
  if chosen_component != component {
    emit(
      window,
      "java",
      format!(
        "Java runtime {component} not found. Using {chosen_component} instead."
      ),
      None,
      None
    )?;
  }

  let entry_list = platform
    .get(&chosen_component)
    .and_then(|value| value.as_array())
    .ok_or_else(|| "No Java runtime components available for this platform.".to_string())?;
  let entry = entry_list
    .iter()
    .find_map(|value| value.as_object())
    .ok_or_else(|| "No Java runtime entries available for this platform.".to_string())?;
  let manifest_url = entry
    .get("manifest")
    .and_then(|value| value.get("url"))
    .and_then(|value| value.as_str())
    .ok_or_else(|| format!("Java runtime manifest url missing for {chosen_component}"))?;

  emit(
    window,
    "java",
    format!("Downloading Java runtime ({component})"),
    None,
    None
  )?;

  let runtime_manifest: JavaRuntimeFiles = fetch_json(&client, manifest_url).await?;
  let runtime_base = game_dir.join("runtime").join(component).join(os_key);
  let runtime_home = runtime_base.join(component);
  ensure_dir(&runtime_home)?;

  let mut downloads: Vec<(Download, PathBuf, bool)> = Vec::new();
  let mut links: Vec<(PathBuf, PathBuf)> = Vec::new();

  for (relative_path, file) in runtime_manifest.files.iter() {
    let out_path = runtime_home.join(relative_path);

    match file.kind.as_str() {
      "directory" => {
        ensure_dir(&out_path)?;
      }
      "file" => {
        let download = file
          .downloads
          .as_ref()
          .and_then(|d| d.raw.as_ref())
          .ok_or_else(|| {
            format!("Missing raw download for Java runtime file {relative_path}")
          })?;
        downloads.push((download.clone(), out_path, file.executable));
      }
      "link" => {
        if let Some(target) = &file.target {
          let base = out_path.parent().unwrap_or(&runtime_home);
          let target_path = base.join(target);
          links.push((target_path, out_path));
        }
      }
      _ => {}
    }
  }

  let total = downloads.len() as u64;
  let mut index = 0u64;
  if total > 0 {
    let mut stream = stream::iter(downloads.into_iter().map(|(download, path, executable)| {
      let client = client.clone();
      async move {
        download_if_needed(&client, &download, &path).await?;
        if executable {
          set_executable(&path)?;
        }
        Ok::<(), String>(())
      }
    }))
    .buffer_unordered(DOWNLOAD_CONCURRENCY);

    while let Some(result) = stream.next().await {
      result?;
      index += 1;
      if index % 200 == 0 || index == total {
        emit(
          window,
          "java",
          format!("Java runtime files {index}/{total}"),
          Some(index),
          Some(total)
        )?;
      }
    }
  }

  for (target, link) in links {
    create_runtime_link(&target, &link)?;
  }

  let java_path = locate_java_binary(&runtime_home, &runtime_manifest);
  if !java_path.exists() {
    return Err("Java runtime download completed but java binary was not found.".to_string());
  }

  Ok(java_path.to_string_lossy().to_string())
}

fn locate_java_binary(runtime_home: &Path, manifest: &JavaRuntimeFiles) -> PathBuf {
  if cfg!(target_os = "windows") {
    let javaw = runtime_home.join("bin").join("javaw.exe");
    if javaw.exists() {
      return javaw;
    }
    let java = runtime_home.join("bin").join("java.exe");
    if java.exists() {
      return java;
    }
  } else {
    let java = runtime_home.join("bin").join("java");
    if java.exists() {
      return java;
    }
  }

  for (relative_path, file) in manifest.files.iter() {
    if file.kind != "file" || !file.executable {
      continue;
    }
    let lower = relative_path.to_lowercase();
    if lower.ends_with("/bin/java") || lower.ends_with("\\bin\\java") {
      return runtime_home.join(relative_path);
    }
    if cfg!(target_os = "windows") {
      if lower.ends_with("/bin/java.exe") || lower.ends_with("\\bin\\java.exe") {
        return runtime_home.join(relative_path);
      }
      if lower.ends_with("/bin/javaw.exe") || lower.ends_with("\\bin\\javaw.exe") {
        return runtime_home.join(relative_path);
      }
    }
  }

  runtime_home.join("bin").join("java")
}

fn set_executable(path: &Path) -> Result<(), String> {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
      .map_err(|err| format!("Failed to read permissions: {err}"))?
      .permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)
      .map_err(|err| format!("Failed to set executable permission: {err}"))?;
  }
  Ok(())
}

fn create_runtime_link(target: &Path, link: &Path) -> Result<(), String> {
  if link.exists() {
    return Ok(());
  }
  if let Some(parent) = link.parent() {
    ensure_dir(parent)?;
  }
  if !target.exists() {
    return Err(format!(
      "Java runtime link target missing: {}",
      target.display()
    ));
  }

  if try_create_symlink(target, link).is_ok() {
    return Ok(());
  }

  if target.is_file() {
    fs::copy(target, link)
      .map_err(|err| format!("Failed to copy Java runtime link: {err}"))?;
    return Ok(());
  }

  if target.is_dir() {
    ensure_dir(link)?;
  }
  Ok(())
}

fn try_create_symlink(target: &Path, link: &Path) -> Result<(), String> {
  #[cfg(unix)]
  {
    std::os::unix::fs::symlink(target, link)
      .map_err(|err| format!("Failed to create symlink: {err}"))?;
    return Ok(());
  }
  #[cfg(windows)]
  {
    if target.is_dir() {
      std::os::windows::fs::symlink_dir(target, link)
        .map_err(|err| format!("Failed to create symlink: {err}"))?;
    } else {
      std::os::windows::fs::symlink_file(target, link)
        .map_err(|err| format!("Failed to create symlink: {err}"))?;
    }
    return Ok(());
  }
  #[allow(unreachable_code)]
  Err("Symlinks are not supported on this platform.".to_string())
}
