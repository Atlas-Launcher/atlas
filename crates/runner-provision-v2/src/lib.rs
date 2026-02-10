pub mod errors;

pub mod deps;
pub mod apply;
pub mod launch;

pub use deps::provider::DependencyProvider;

pub use apply::ensure_applied_from_packblob_bytes;
pub use launch::LaunchPlan;

pub(crate) fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis()
}