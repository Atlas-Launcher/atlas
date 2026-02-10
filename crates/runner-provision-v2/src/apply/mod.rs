use std::path::{Path, PathBuf};

use protocol::{decode_blob, PackBlob};

use crate::{
    deps::{provider::DependencyProvider, verify},
    errors::ProvisionError,
    launch::{self, LaunchPlan},
};

mod plan;
mod staging;
mod preserve;
mod marker;
mod pointers;
mod loader;

pub async fn ensure_applied_from_packblob_bytes(
    server_root: &Path,
    pack_blob_bytes: &[u8],
    dep_provider: &dyn DependencyProvider,
) -> Result<LaunchPlan, ProvisionError> {
    // 1) Decode PackBlob
    let pack = decode_packblob(pack_blob_bytes)?;

    // 2) Short-circuit if already applied
    if marker::is_pack_applied(server_root, &pack).await? {
        return launch::read_launch_plan(server_root).await;
    }

    // 3) Build apply plan (what files to write where)
    let plan = plan::build_apply_plan(&pack)?;

    // 4) Stage everything into staging dir
    let staging_dir = staging::create_staging_dir(server_root).await?;
    let staging_current = staging_dir.join("current"); // staging/current/...

    staging::ensure_dir(&staging_current).await?;

    // 4a) Write inline files
    plan::write_inline_files(&pack, &staging_current).await?;

    // 4b) Fetch+verify+write dependencies
    for item in plan.deps {
        let bytes = dep_provider.fetch(&item.dep).await?;
        verify::verify_dependency_bytes(&item.dep, &bytes)?;
        plan::write_dependency_bytes(&item, &bytes, &staging_current).await?;
    }

    // 4c) Ensure server loader is installed
    loader::ensure_loader_installed(server_root, &staging_current, &pack.metadata).await?;

    // 5) Preserve selected files from existing current -> staging/current
    preserve::preserve_from_existing(server_root, &staging_current).await?;

    // 6) Write launch plan + applied marker into staging/current/.runner/
    let launch_plan = launch::derive_launch_plan(&pack, &staging_current)?;
    launch::write_launch_plan_to_dir(&staging_current, &launch_plan).await?;
    marker::write_applied_marker_to_dir(&staging_current, &pack).await?;

    // 7) Promote staging/current to server_root/current atomically
    staging::promote(server_root, &staging_current).await?;

    Ok(launch_plan)
}

fn decode_packblob(bytes: &[u8]) -> Result<PackBlob, ProvisionError> {
    use prost::Message;
    decode_blob(bytes).map_err(|e| ProvisionError::Invalid(format!("Failed to decode PackBlob: {e}")))
}
