use std::sync::Mutex;

use crate::models::AuthSession;

pub struct AppState {
  pub auth: Mutex<Option<AuthSession>>
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      auth: Mutex::new(None)
    }
  }
}
