use crate::paths::{ensure_dir, file_exists};
use futures::StreamExt;
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use super::manifest::Download;

pub const DOWNLOAD_CONCURRENCY: usize = 12;

pub async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, String> {
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

pub async fn download_if_needed(
  client: &Client,
  download: &Download,
  path: &Path
) -> Result<(), String> {
  let mut allow_resume = true;
  if file_exists(path) {
    if let Some(expected) = &download.sha1 {
      if let Ok(actual) = sha1_file(path) {
        if &actual == expected {
          return Ok(());
        }
      }
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

pub async fn download_raw(
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
  let mut file = std::fs::File::open(path).map_err(|err| format!("Failed to open file: {err}"))?;
  let mut hasher = Sha1::new();
  let mut buffer = [0u8; 8192];
  loop {
    let read = std::io::Read::read(&mut file, &mut buffer)
      .map_err(|err| format!("Read failed: {err}"))?;
    if read == 0 {
      break;
    }
    hasher.update(&buffer[..read]);
  }
  Ok(hex::encode(hasher.finalize()))
}
