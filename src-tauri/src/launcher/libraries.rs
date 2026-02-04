use crate::paths::ensure_dir;
use std::fs::File;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use super::args::rules_allow;
use super::download::{download_if_needed, DOWNLOAD_CONCURRENCY};
use super::manifest::Library;
use super::emit;
use futures::stream::{self, StreamExt};
use reqwest::Client;

pub async fn sync_libraries(
  client: &Client,
  libraries_dir: &Path,
  libraries: &[Library],
  window: &tauri::Window
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
  let mut library_paths = Vec::new();
  let mut native_paths = Vec::new();
  let mut downloads: Vec<(super::manifest::Download, PathBuf)> = Vec::new();

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

pub fn extract_natives(
  path: &Path,
  natives_dir: &Path,
  libraries: &[Library]
) -> Result<(), String> {
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
    std::io::copy(&mut entry, &mut outfile)
      .map_err(|err| format!("Failed to extract native: {err}"))?;
  }

  Ok(())
}

pub fn build_classpath(libraries: &[PathBuf], client_jar: &Path) -> String {
  let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
  let mut entries: Vec<String> =
    libraries.iter().map(|path| path.to_string_lossy().to_string()).collect();
  entries.push(client_jar.to_string_lossy().to_string());
  entries.join(sep)
}

pub(crate) fn library_path_from_name(name: &str) -> String {
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

pub fn current_os_key() -> &'static str {
  if cfg!(target_os = "windows") {
    "windows"
  } else if cfg!(target_os = "macos") {
    "osx"
  } else {
    "linux"
  }
}

pub fn current_arch() -> &'static str {
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
