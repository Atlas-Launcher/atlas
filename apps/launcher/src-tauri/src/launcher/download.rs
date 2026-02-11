use crate::paths::{ensure_dir, file_exists};
use futures::StreamExt;
use reqwest::header::RANGE;
use reqwest::{Client, StatusCode};
use sha1::{Digest, Sha1};
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};

use super::manifest::Download;

pub const DOWNLOAD_CONCURRENCY: usize = 12;
const DOWNLOAD_MAX_RETRIES: usize = 3;

#[derive(Debug, Clone)]
pub struct DownloadRetryEvent {
    pub attempt: usize,
    pub max_attempts: usize,
    pub delay_ms: u64,
    pub reason: String,
}

pub async fn download_if_needed(
    client: &Client,
    download: &Download,
    path: &Path,
) -> Result<(), String> {
    download_if_needed_with_retry_events(client, download, path, |_| {}).await
}

pub async fn download_if_needed_with_retry_events<F>(
    client: &Client,
    download: &Download,
    path: &Path,
    mut on_retry: F,
) -> Result<(), String>
where
    F: FnMut(DownloadRetryEvent),
{
    let mut allow_resume = true;
    if file_exists(path) {
        if download.sha1.is_none() && download.size.is_none() {
            return Ok(());
        }
        if let Some(expected) = &download.sha1 {
            if let Ok(actual) = sha1_file(path) {
                if actual.eq_ignore_ascii_case(expected) {
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

    download_raw_with_retry_events(
        client,
        &download.url,
        path,
        download.size,
        allow_resume,
        &mut on_retry,
    )
    .await?;

    if let Some(expected) = &download.sha1 {
        let actual = sha1_file(path)?;
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(path);
            return Err(format!(
                "Downloaded file hash mismatch for {}: expected {}, got {}",
                path.display(),
                expected,
                actual
            ));
        }
    }

    Ok(())
}

pub async fn download_raw(
    client: &Client,
    url: &str,
    path: &Path,
    expected_size: Option<u64>,
    allow_resume: bool,
) -> Result<(), String> {
    download_raw_with_retry_events(client, url, path, expected_size, allow_resume, |_| {}).await
}

pub async fn download_raw_with_retry_events<F>(
    client: &Client,
    url: &str,
    path: &Path,
    expected_size: Option<u64>,
    allow_resume: bool,
    mut on_retry: F,
) -> Result<(), String>
where
    F: FnMut(DownloadRetryEvent),
{
    let mut existing = if allow_resume && file_exists(path) {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
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

    let mut response = send_with_retries(
        client,
        url,
        if allow_resume && existing > 0 {
            Some(format!("bytes={}-", existing))
        } else {
            None
        },
        &mut on_retry,
    )
    .await?;

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
                response = send_with_retries(client, url, None, &mut on_retry).await?;
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

    let mut file =
        if allow_resume && existing > 0 && response.status() == StatusCode::PARTIAL_CONTENT {
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
        file.write_all(&bytes)
            .await
            .map_err(|err| format!("Failed to write file: {err}"))?;
    }

    file.flush()
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

fn retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
        || status.is_server_error()
}

fn retryable_error(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}

fn with_jitter(base: Duration) -> Duration {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let jitter_ms = nanos % 180;
    base + Duration::from_millis(jitter_ms)
}

async fn send_with_retries(
    client: &Client,
    url: &str,
    range_header: Option<String>,
    on_retry: &mut impl FnMut(DownloadRetryEvent),
) -> Result<reqwest::Response, String> {
    let mut backoff = Duration::from_millis(250);
    for attempt in 0..=DOWNLOAD_MAX_RETRIES {
        let mut request = client.get(url);
        if let Some(range) = range_header.as_ref() {
            request = request.header(RANGE, range.clone());
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success()
                    || status == StatusCode::PARTIAL_CONTENT
                    || status == StatusCode::RANGE_NOT_SATISFIABLE
                {
                    return Ok(response);
                }
                if retryable_status(status) && attempt < DOWNLOAD_MAX_RETRIES {
                    let delay = with_jitter(backoff);
                    on_retry(DownloadRetryEvent {
                        attempt: attempt + 1,
                        max_attempts: DOWNLOAD_MAX_RETRIES + 1,
                        delay_ms: delay.as_millis() as u64,
                        reason: format!("retryable status {status}"),
                    });
                    sleep(delay).await;
                    backoff = (backoff * 2).min(Duration::from_secs(2));
                    continue;
                }
                let text = response.text().await.unwrap_or_default();
                return Err(format!("Download failed ({status}): {text}"));
            }
            Err(err) => {
                if retryable_error(&err) && attempt < DOWNLOAD_MAX_RETRIES {
                    let delay = with_jitter(backoff);
                    on_retry(DownloadRetryEvent {
                        attempt: attempt + 1,
                        max_attempts: DOWNLOAD_MAX_RETRIES + 1,
                        delay_ms: delay.as_millis() as u64,
                        reason: format!("transient error: {err}"),
                    });
                    sleep(delay).await;
                    backoff = (backoff * 2).min(Duration::from_secs(2));
                    continue;
                }
                return Err(format!("Download failed: {err}"));
            }
        }
    }

    Err("Download failed after retries.".to_string())
}

fn sha1_file(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|err| format!("Failed to open file: {err}"))?;
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
