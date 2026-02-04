use super::http::HttpClient;
use super::ms;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::sync::Mutex;

struct MockHttp {
    responses: Mutex<Vec<serde_json::Value>>,
    form_calls: Mutex<Vec<(String, Vec<(String, String)>)>>,
}

impl MockHttp {
    fn new(responses: Vec<serde_json::Value>) -> Self {
        Self {
            responses: Mutex::new(responses),
            form_calls: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl HttpClient for MockHttp {
    async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> Result<T, String> {
        self.form_calls.lock().unwrap().push((
            url.to_string(),
            params
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        ));
        let value = self.responses.lock().unwrap().remove(0);
        serde_json::from_value(value).map_err(|err| err.to_string())
    }

    async fn post_json<T: DeserializeOwned, B: serde::Serialize + Send + Sync>(
        &self,
        _url: &str,
        _body: &B,
    ) -> Result<T, String> {
        Err("post_json not implemented in mock".to_string())
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        _url: &str,
        _bearer: Option<&str>,
    ) -> Result<T, String> {
        Err("get_json not implemented in mock".to_string())
    }
}

#[tokio::test]
async fn start_device_code_uses_form_params() {
    let http = MockHttp::new(vec![json!({
      "device_code": "device",
      "user_code": "user",
      "verification_uri": "https://example.com",
      "expires_in": 900,
      "interval": 5
    })]);

    let response = ms::start_device_code(&http, "client").await.unwrap();
    assert_eq!(response.device_code, "device");
    assert_eq!(response.user_code, "user");

    let calls = http.form_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let params = &calls[0].1;
    assert!(params
        .iter()
        .any(|(k, v)| k == "client_id" && v == "client"));
    assert!(params
        .iter()
        .any(|(k, v)| k == "scope" && v.contains("XboxLive.signin")));
}

#[tokio::test]
async fn refresh_token_uses_form_params() {
    let http = MockHttp::new(vec![json!({
      "access_token": "access",
      "refresh_token": "refresh",
      "expires_in": 3600,
      "token_type": "Bearer",
      "scope": "XboxLive.signin offline_access"
    })]);

    let response = ms::refresh_token(&http, "client", "refresh").await.unwrap();
    assert_eq!(response.access_token, "access");

    let calls = http.form_calls.lock().unwrap();
    let params = &calls[0].1;
    assert!(params
        .iter()
        .any(|(k, v)| k == "grant_type" && v == "refresh_token"));
}

#[test]
fn parse_auth_callback_accepts_valid_response() {
    let url = "atlas://auth?code=abc123&state=state1";
    let code = ms::parse_auth_callback(url, "state1").unwrap();
    assert_eq!(code, "abc123");
}

#[test]
fn parse_auth_callback_rejects_error() {
    let url = "atlas://auth?error=access_denied&state=state1";
    let err = ms::parse_auth_callback(url, "state1").unwrap_err();
    assert!(err.contains("Microsoft sign-in failed"));
}

#[test]
fn parse_auth_callback_rejects_state_mismatch() {
    let url = "atlas://auth?code=abc123&state=wrong";
    let err = ms::parse_auth_callback(url, "state1").unwrap_err();
    assert!(err.contains("state did not match"));
}
