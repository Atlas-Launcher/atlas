use async_trait::async_trait;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[async_trait]
pub trait HttpClient: Send + Sync {
  async fn post_form<T: DeserializeOwned>(
    &self,
    url: &str,
    params: &[(&str, &str)]
  ) -> Result<T, String>;

  async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
    &self,
    url: &str,
    body: &B
  ) -> Result<T, String>;

  async fn get_json<T: DeserializeOwned>(
    &self,
    url: &str,
    bearer: Option<&str>
  ) -> Result<T, String>;
}

pub struct ReqwestHttpClient {
  client: Client
}

impl ReqwestHttpClient {
  pub fn new() -> Self {
    Self {
      client: Client::new()
    }
  }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
  async fn post_form<T: DeserializeOwned>(
    &self,
    url: &str,
    params: &[(&str, &str)]
  ) -> Result<T, String> {
    let response = self
      .client
      .post(url)
      .form(&params)
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
      .map_err(|err| format!("Failed to parse response: {err}"))
  }

  async fn post_json<T: DeserializeOwned, B: Serialize + Send + Sync>(
    &self,
    url: &str,
    body: &B
  ) -> Result<T, String> {
    let response = self
      .client
      .post(url)
      .json(body)
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
      .map_err(|err| format!("Failed to parse response: {err}"))
  }

  async fn get_json<T: DeserializeOwned>(
    &self,
    url: &str,
    bearer: Option<&str>
  ) -> Result<T, String> {
    let mut request = self.client.get(url);
    if let Some(token) = bearer {
      request = request.bearer_auth(token);
    }

    let response = request.send().await.map_err(|err| format!("Request failed: {err}"))?;

    if !response.status().is_success() {
      let status = response.status();
      let text = response.text().await.unwrap_or_default();
      return Err(format!("Request failed ({status}): {text}"));
    }

    response
      .json::<T>()
      .await
      .map_err(|err| format!("Failed to parse response: {err}"))
  }
}
