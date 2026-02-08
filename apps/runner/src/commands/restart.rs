use anyhow::Result;
use crate::commands::{down, up};

pub async fn exec() -> Result<()> {
    down::exec().await?;
    up::exec(false).await?;
    Ok(())
}
