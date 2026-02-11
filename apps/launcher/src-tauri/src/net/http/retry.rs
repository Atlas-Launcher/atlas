use reqwest::{Client, StatusCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

use super::errors::HttpError;

const HTTP_MAX_RETRIES: usize = 3;

pub async fn get_with_retries(client: &Client, url: &str) -> Result<reqwest::Response, HttpError> {
    let mut backoff = Duration::from_millis(250);
    for attempt in 0..=HTTP_MAX_RETRIES {
        match client.get(url).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    return Ok(response);
                }
                if retryable_status(status) && attempt < HTTP_MAX_RETRIES {
                    sleep(with_jitter(backoff)).await;
                    backoff = (backoff * 2).min(Duration::from_secs(3));
                    continue;
                }
                let body = response.text().await.unwrap_or_default();
                return Err(HttpError::Status { status, body });
            }
            Err(err) => {
                if retryable_error(&err) && attempt < HTTP_MAX_RETRIES {
                    sleep(with_jitter(backoff)).await;
                    backoff = (backoff * 2).min(Duration::from_secs(3));
                    continue;
                }
                return Err(HttpError::Request(err));
            }
        }
    }

    Err(HttpError::Status {
        status: StatusCode::REQUEST_TIMEOUT,
        body: "Request failed after retries.".to_string(),
    })
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
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let jitter_ms = nanos % 180;
    base + Duration::from_millis(jitter_ms)
}

#[cfg(test)]
mod tests {
    use super::retryable_status;
    use reqwest::StatusCode;

    #[test]
    fn retryable_status_classification_is_expected() {
        assert!(retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(retryable_status(StatusCode::BAD_GATEWAY));
        assert!(!retryable_status(StatusCode::NOT_FOUND));
        assert!(!retryable_status(StatusCode::UNAUTHORIZED));
    }
}
