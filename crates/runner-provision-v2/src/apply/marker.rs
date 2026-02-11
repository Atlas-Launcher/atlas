use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::errors::ProvisionError;
use protocol::{Loader, PackBlob};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedMarker {
    pub pack_id: String,
    pub version: String,
    pub minecraft_version: String,
    pub loader: Loader,
}

pub async fn is_pack_applied(server_root: &Path, pack: &PackBlob) -> Result<bool, ProvisionError> {
    let want = marker_from_pack(pack)?;
    let path = server_root
        .join("current")
        .join(".runner")
        .join("applied.json");

    if !tokio::fs::try_exists(&path).await? {
        return Ok(false);
    }
    let bytes = tokio::fs::read(&path).await?;
    let have: AppliedMarker = serde_json::from_slice(&bytes)?;
    Ok(have.pack_id == want.pack_id
        && have.version == want.version
        && have.minecraft_version == want.minecraft_version
        && have.loader == want.loader)
}

pub async fn write_applied_marker_to_dir(
    staging_current: &Path,
    pack: &PackBlob,
) -> Result<(), ProvisionError> {
    let marker = marker_from_pack(pack)?;
    let dir = staging_current.join(".runner");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join("applied.json");
    let bytes = serde_json::to_vec_pretty(&marker)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

fn marker_from_pack(pack: &PackBlob) -> Result<AppliedMarker, ProvisionError> {
    let meta = pack.metadata.as_ref();
    Ok(AppliedMarker {
        pack_id: meta.pack_id.clone(),
        version: meta.version.clone(),
        minecraft_version: meta.minecraft_version.clone(),
        loader: meta.loader.clone(),
    })
}
