use anyhow::Result;
use std::path::{Path, PathBuf};

const LEGACY_JAVA_ROOT: &str = "/var/lib/atlas-runner/java";

pub async fn ensure_java_for_minecraft(
    mc_version: &str,
    override_major: Option<u32>,
) -> Result<PathBuf> {
    runner_provision_v2::java::ensure_java_for_minecraft_with_root(
        Path::new(LEGACY_JAVA_ROOT),
        mc_version,
        override_major,
    )
    .await
    .map_err(anyhow::Error::from)
}
