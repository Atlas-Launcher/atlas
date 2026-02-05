use reqwest::Client;

use super::errors::HttpError;

pub async fn fetch_text(client: &Client, url: &str) -> Result<String, HttpError> {
    let response = client.get(url).send().await.map_err(HttpError::Request)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(HttpError::Status { status, body: text });
    }

    response.text().await.map_err(HttpError::Request)
}
