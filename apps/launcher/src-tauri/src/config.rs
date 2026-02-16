use crate::models::AppSettings;

pub const DEFAULT_MS_CLIENT_ID: &str = option_env!("ATLAS_MS_CLIENT_ID")
    .unwrap_or("atlas-ms-client-id-not-configured");
pub const DEFAULT_REDIRECT_URI: &str = "atlas://auth";
pub const DEFAULT_ATLAS_HUB_URL: &str = atlas_client::device_code::DEFAULT_ATLAS_HUB_URL;
pub const DEFAULT_ATLAS_CLIENT_ID: &str = atlas_client::device_code::DEFAULT_ATLAS_DEVICE_CLIENT_ID;
pub const DEFAULT_ATLAS_REDIRECT_URI: &str = "atlas://signin";

pub fn resolve_client_id(settings: &AppSettings) -> String {
    settings
        .ms_client_id
        .as_ref()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_MS_CLIENT_ID.to_string())
}

pub fn resolve_atlas_hub_url(settings: &AppSettings) -> String {
    settings
        .atlas_hub_url
        .as_ref()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ATLAS_HUB_URL.to_string())
}

pub fn resolve_atlas_client_id() -> String {
    DEFAULT_ATLAS_CLIENT_ID.to_string()
}

pub fn resolve_atlas_redirect_uri() -> String {
    DEFAULT_ATLAS_REDIRECT_URI.to_string()
}

pub fn resolve_atlas_auth_base_url(settings: &AppSettings) -> String {
    format!("{}/api/auth", resolve_atlas_hub_url(settings))
}
