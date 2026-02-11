use reqwest::Client;
use serde::de::DeserializeOwned;

use super::client::shared_client;
use super::errors::HttpError;
use super::retry::get_with_retries;

pub async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, HttpError> {
    let response = get_with_retries(client, url).await?;

    let body = response.text().await.map_err(HttpError::Request)?;
    serde_json::from_str::<T>(&body).map_err(|err| HttpError::Parse { source: err, body })
}

pub async fn fetch_json_shared<T: DeserializeOwned>(url: &str) -> Result<T, HttpError> {
    fetch_json(shared_client(), url).await
}
