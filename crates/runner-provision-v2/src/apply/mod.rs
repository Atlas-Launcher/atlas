use std::path::{Path, PathBuf};

use protocol::{decode_blob, PackBlob};

use crate::{
    deps::{provider::DependencyProvider, verify},
    errors::ProvisionError,
    java,
    launch::{self, LaunchPlan},
};

mod plan;
mod staging;
mod preserve;
mod marker;
mod pointers;
mod loader;
mod eula;
mod server_properties;

pub async fn ensure_applied_from_packblob_bytes(
    server_root: &Path,
    pack_blob_bytes: &[u8],
    dep_provider: &dyn DependencyProvider,
) -> Result<LaunchPlan, ProvisionError> {
    // 1) Decode PackBlob
    let pack = decode_packblob(pack_blob_bytes)?;

    // 2) Ensure java runtime is available
    let java_bin = java::ensure_java_for_minecraft(
        server_root,
        &pack.metadata.minecraft_version,
        None,
    )
    .await?;

    // 3) Short-circuit if already applied
    if marker::is_pack_applied(server_root, &pack).await? {
        let mut plan = launch::read_launch_plan(server_root).await?;
        launch::apply_java_path_to_plan(&mut plan, &java_bin);
        let current_dir = server_root.join("current");
        eula::ensure_eula(&current_dir).await?;
        server_properties::ensure_whitelist_enforced(&current_dir).await?;
        server_properties::ensure_rcon_configured(&current_dir).await?;
        launch::write_launch_plan_to_dir(&current_dir, &plan).await?;
        return Ok(plan);
    }

    // 4) Build apply plan (what files to write where)
    let plan = plan::build_apply_plan(&pack)?;

    // 5) Stage everything into staging dir
    let staging_dir = staging::create_staging_dir(server_root).await?;
    let staging_current = staging_dir.join("current"); // staging/current/...

    staging::ensure_dir(&staging_current).await?;

    // 5a) Write inline files
    plan::write_inline_files(&pack, &staging_current).await?;

    // 5b) Fetch+verify+write dependencies
    for item in plan.deps {
        let bytes = dep_provider.fetch(&item.dep).await?;
        verify::verify_dependency_bytes(&item.dep, &bytes)?;
        plan::write_dependency_bytes(&item, &bytes, &staging_current).await?;
    }

    // 5c) Ensure server loader is installed
    loader::ensure_loader_installed(
        server_root,
        &staging_current,
        &pack.metadata,
        &java_bin,
    )
    .await?;

    // 6) Preserve selected files from existing current -> staging/current
    preserve::preserve_from_existing(server_root, &staging_current).await?;

    // 7) Ensure EULA + whitelist + RCON are enforced
    eula::ensure_eula(&staging_current).await?;
    server_properties::ensure_whitelist_enforced(&staging_current).await?;
    server_properties::ensure_rcon_configured(&staging_current).await?;

    // 8) Write launch plan + applied marker into staging/current/.runner/
    let launch_plan = launch::derive_launch_plan(&pack, &staging_current, &java_bin)?;
    launch::write_launch_plan_to_dir(&staging_current, &launch_plan).await?;
    marker::write_applied_marker_to_dir(&staging_current, &pack).await?;

    // 9) Promote staging/current to server_root/current atomically
    staging::promote(server_root, &staging_current).await?;

    Ok(launch_plan)
}

fn decode_packblob(bytes: &[u8]) -> Result<PackBlob, ProvisionError> {
    use prost::Message;
    decode_blob(bytes).map_err(|e| ProvisionError::Invalid(format!("Failed to decode PackBlob: {e}")))
}
