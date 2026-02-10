use async_trait::async_trait;

use crate::errors::ProvisionError;
use protocol::Dependency;

#[async_trait]
pub trait DependencyProvider: Send + Sync {
    /// Return the raw bytes for this dependency URL.
    /// The provisioner will verify hash + place it into the correct target path.
    async fn fetch(&self, dep: &Dependency) -> Result<Vec<u8>, ProvisionError>;
}
