use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T, String>;

    async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, String>;

    async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<T, String>;
}

pub struct ReqwestHttpClient {
    client: Client,
}

impl ReqwestHttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T, String> {
        let response = self
            .client
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(|err| format!("Request failed: {err}"))?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|err| format!("Failed to read response: {err}"))?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(format!("Request failed ({status}): {text}"));
        }

        serde_json::from_slice::<T>(&body).map_err(|err| {
            let text = String::from_utf8_lossy(&body);
            format!("Failed to parse response: {err}. Body: {text}")
        })
    }

    async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T, String> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|err| format!("Request failed: {err}"))?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|err| format!("Failed to read response: {err}"))?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(format!("Request failed ({status}): {text}"));
        }

        serde_json::from_slice::<T>(&body).map_err(|err| {
            let text = String::from_utf8_lossy(&body);
            format!("Failed to parse response: {err}. Body: {text}")
        })
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        bearer: Option<&str>,
    ) -> Result<T, String> {
        let mut request = self.client.get(url);
        if let Some(token) = bearer {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|err| format!("Request failed: {err}"))?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|err| format!("Failed to read response: {err}"))?;
        if !status.is_success() {
            let text = String::from_utf8_lossy(&body);
            return Err(format!("Request failed ({status}): {text}"));
        }

        serde_json::from_slice::<T>(&body).map_err(|err| {
            let text = String::from_utf8_lossy(&body);
            format!("Failed to parse response: {err}. Body: {text}")
        })
    }
}
