use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::OnceLock;

use super::errors::HttpError;

static CLIENT: OnceLock<Client> = OnceLock::new();

pub fn shared_client() -> &'static Client {
    CLIENT.get_or_init(|| Client::new())
}

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T, HttpError>;

    async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, HttpError>;

    async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<T, HttpError>;
}

pub struct ReqwestHttpClient {
    client: Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: shared_client().clone(),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T, HttpError> {
        let response = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(HttpError::Request)?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(HttpError::Request)?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(HttpError::Status {
                status,
                body: text.to_string(),
            });
        }

        serde_json::from_slice::<T>(&body).map_err(|err| HttpError::Parse {
            source: err,
            body: String::from_utf8_lossy(&body).to_string(),
        })
    }

    async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, HttpError> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(HttpError::Request)?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(HttpError::Request)?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(HttpError::Status {
                status,
                body: text.to_string(),
            });
        }

        serde_json::from_slice::<T>(&body).map_err(|err| HttpError::Parse {
            source: err,
            body: String::from_utf8_lossy(&body).to_string(),
        })
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<T, HttpError> {
        let mut request = self.client.get(url);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(HttpError::Request)?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(HttpError::Request)?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(HttpError::Status {
                status,
                body: text.to_string(),
            });
        }

        serde_json::from_slice::<T>(&body).map_err(|err| HttpError::Parse {
            source: err,
            body: String::from_utf8_lossy(&body).to_string(),
        })
    }
}
