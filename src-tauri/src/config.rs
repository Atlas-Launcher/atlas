use crate::models::AppSettings;

pub const DEFAULT_MS_CLIENT_ID: &str = "REDACTED-MS-CLIENT-ID";
pub const DEFAULT_REDIRECT_URI: &str = "atlas://auth";

pub fn resolve_client_id(settings: &AppSettings) -> String {
    settings
        .ms_client_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_MS_CLIENT_ID.to_string())
}
