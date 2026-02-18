use crate::telemetry;
use atlas_client::device_code::{
    hub_device_code_endpoint, hub_device_token_endpoint, parse_device_token_poll_body,
    DeviceCodeRequest, DeviceTokenPollStatus, DeviceTokenRequest, StandardDeviceTokenResponse,
};
use atlas_client::oauth as client_oauth;
use std::time::{Duration, Instant};
use tokio::time::sleep;

use super::error::AuthError;

pub use client_oauth::{AtlasTokenResponse, AtlasUserInfo, AuthRequest};

pub(crate) async fn start_device_code(
    hub_url: &str,
    client_id: &str,
) -> Result<atlas_client::device_code::DeviceCodeResponse, AuthError> {
    let url = hub_device_code_endpoint(hub_url);
    let request = DeviceCodeRequest {
        client_id,
        scope: atlas_client::device_code::DEFAULT_ATLAS_DEVICE_SCOPE,
    };

    telemetry::info(format!(
        "Atlas device code start requested (hub_url={hub_url}, client_id={client_id})."
    ));

    let response: atlas_client::device_code::DeviceCodeResponse = reqwest::Client::new()
        .post(url)
        .json(&request)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?
        .json()
        .await
        .map_err(|err| err.to_string())?;

    telemetry::info(format!(
        "Atlas device code start succeeded (expires_in={}s, interval={}s).",
        response.expires_in, response.interval
    ));
    Ok(response)
}

pub(crate) async fn poll_device_token(
    hub_url: &str,
    client_id: &str,
    device_code: &str,
    interval_seconds: u64,
) -> Result<StandardDeviceTokenResponse, AuthError> {
    let poll_url = hub_device_token_endpoint(hub_url);
    let request = DeviceTokenRequest::new(client_id, device_code);
    let mut interval = Duration::from_secs(interval_seconds.max(1));
    let start = Instant::now();
    let timeout = Duration::from_secs(900);
    let mut attempts: u64 = 0;

    telemetry::info(format!(
        "Atlas device token polling started (hub_url={hub_url}, client_id={client_id}, interval={}s).",
        interval.as_secs()
    ));

    loop {
        attempts = attempts.saturating_add(1);
        if start.elapsed() > timeout {
            telemetry::warn(format!(
                "Atlas device token polling timed out after {} attempts (elapsed={}s).",
                attempts,
                start.elapsed().as_secs()
            ));
            return Err("Atlas device code expired. Start sign-in again."
                .to_string()
                .into());
        }

        let response = reqwest::Client::new()
            .post(&poll_url)
            .json(&request)
            .send()
            .await
            .map_err(|err| err.to_string())?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(|err| err.to_string())?;
        let poll_status =
            parse_device_token_poll_body::<StandardDeviceTokenResponse>(status, &body)
                .map_err(|err| err.to_string())?;

        match poll_status {
            DeviceTokenPollStatus::Success(token) => {
                telemetry::info(format!(
                    "Atlas device token polling succeeded after {} attempts (elapsed={}s).",
                    attempts,
                    start.elapsed().as_secs()
                ));
                return Ok(token);
            }
            DeviceTokenPollStatus::AuthorizationPending => {
                if attempts == 1 || attempts % 5 == 0 {
                    telemetry::info(format!(
                        "Atlas device token polling pending (attempt={}, elapsed={}s).",
                        attempts,
                        start.elapsed().as_secs()
                    ));
                }
                sleep(interval).await;
            }
            DeviceTokenPollStatus::SlowDown => {
                interval += Duration::from_secs(5);
                telemetry::warn(format!(
                    "Atlas device token polling slow_down (attempt={}, new_interval={}s).",
                    attempts,
                    interval.as_secs()
                ));
                sleep(interval).await;
            }
            DeviceTokenPollStatus::ExpiredToken => {
                telemetry::warn(format!(
                    "Atlas device token polling expired_token after {} attempts.",
                    attempts
                ));
                return Err("Atlas device code expired. Start sign-in again."
                    .to_string()
                    .into());
            }
            DeviceTokenPollStatus::AccessDenied => {
                telemetry::warn(format!(
                    "Atlas device token polling access_denied after {} attempts.",
                    attempts
                ));
                return Err("Atlas device code authorization denied.".to_string().into());
            }
            DeviceTokenPollStatus::Fatal(message) => {
                telemetry::error(format!(
                    "Atlas device token polling fatal error after {} attempts: {}",
                    attempts, message
                ));
                return Err(format!("Atlas sign-in failed: {message}").into());
            }
        }
    }
}

pub(crate) fn build_auth_request(
    auth_base_url: &str,
    client_id: &str,
    redirect_uri: &str,
) -> Result<AuthRequest, AuthError> {
    client_oauth::build_auth_request(auth_base_url, client_id, redirect_uri)
        .map_err(|err| err.to_string().into())
}

pub(crate) async fn exchange_auth_code(
    auth_base_url: &str,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<AtlasTokenResponse, AuthError> {
    client_oauth::exchange_auth_code(auth_base_url, client_id, code, redirect_uri, code_verifier)
        .await
        .map_err(|err| err.to_string().into())
}

pub(crate) async fn refresh_token(
    auth_base_url: &str,
    client_id: &str,
    refresh_token: &str,
) -> Result<AtlasTokenResponse, AuthError> {
    client_oauth::refresh_token(auth_base_url, client_id, refresh_token)
        .await
        .map_err(|err| err.to_string().into())
}

pub(crate) async fn fetch_user_info(
    auth_base_url: &str,
    access_token: &str,
) -> Result<AtlasUserInfo, AuthError> {
    client_oauth::fetch_user_info(auth_base_url, access_token)
        .await
        .map_err(|err| err.to_string().into())
}

pub(crate) fn parse_auth_callback(
    callback_url: &str,
    expected_state: &str,
) -> Result<String, AuthError> {
    client_oauth::parse_auth_callback(callback_url, expected_state)
        .map_err(|err| err.to_string().into())
}
