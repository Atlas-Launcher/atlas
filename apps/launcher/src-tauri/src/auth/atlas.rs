use atlas_client::oauth as client_oauth;

use super::error::AuthError;

pub use client_oauth::{AtlasTokenResponse, AtlasUserInfo, AuthRequest};

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
