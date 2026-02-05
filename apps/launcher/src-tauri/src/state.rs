use std::sync::Mutex;

use crate::auth::{AtlasPendingAuth, PendingAuth};
use crate::models::{AppSettings, AtlasSession, AuthSession};
use crate::settings;

pub struct AppState {
    pub auth: Mutex<Option<AuthSession>>,
    pub pending_auth: Mutex<Option<PendingAuth>>,
    pub atlas_auth: Mutex<Option<AtlasSession>>,
    pub pending_atlas_auth: Mutex<Option<AtlasPendingAuth>>,
    pub settings: Mutex<AppSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        let settings = settings::load_settings().unwrap_or_default();
        Self {
            auth: Mutex::new(None),
            pending_auth: Mutex::new(None),
            atlas_auth: Mutex::new(None),
            pending_atlas_auth: Mutex::new(None),
            settings: Mutex::new(settings),
        }
    }
}
