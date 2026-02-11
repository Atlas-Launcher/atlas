use anyhow::Result;
use runner_core_v2::proto::{Envelope, Outbound, Request, Response};

use crate::client::connect_or_start;

pub async fn backup_now() -> Result<String> {
    let mut framed = connect_or_start().await?;
    let req = Envelope {
        id: 1,
        payload: Request::Backup {},
    };
    runner_ipc_v2::framing::send_request(&mut framed, &req).await?;

    loop {
        let outbound = runner_ipc_v2::framing::read_outbound(&mut framed)
            .await?
            .ok_or_else(|| anyhow::anyhow!("runnerd closed the connection"))?;
        match outbound {
            Outbound::Response(env) => match env.payload {
                Response::BackupCreated { path } => return Ok(path),
                Response::Error(err) => {
                    return Err(anyhow::anyhow!("backup failed: {}", err.message))
                }
                other => return Err(anyhow::anyhow!("unexpected response: {:?}", other)),
            },
            Outbound::Event(_) => continue,
        }
    }
}
