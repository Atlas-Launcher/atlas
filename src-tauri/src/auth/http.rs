use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub async fn post_json<T: DeserializeOwned, B: Serialize>(
  client: &Client,
  url: &str,
  body: &B
) -> Result<T, String> {
  let response = client
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
