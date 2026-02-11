use reqwest::Client;

use super::errors::HttpError;
use super::retry::get_with_retries;

pub async fn fetch_text(client: &Client, url: &str) -> Result<String, HttpError> {
    let response = get_with_retries(client, url).await?;

    response.text().await.map_err(HttpError::Request)
}
