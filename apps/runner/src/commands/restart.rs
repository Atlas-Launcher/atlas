use crate::commands::{down, up};
use anyhow::Result;

pub async fn exec() -> Result<()> {
    down::exec().await?;
    up::exec(false, true, false).await?;
    Ok(())
}
